//! Issue 153 T1/T2 — the CLI version alert + the verified binary self-update.
//!
//! The alert: one stderr line when a newer `cargo-refine` release exists,
//! backed by a cache so the network is touched at most once per 12 h (45 min
//! after a failed attempt) and never blocks a run by more than the client
//! timeout. Default-on (the owner ask: "the old version not have this yet");
//! `RIIR_REFINE_UPDATE_CHECK=0` opts out; a `CI` env skips (build-agent
//! noise). Fail-soft by construction — every failure path is silent or one
//! stderr line, never a nonzero exit.
//!
//! The self-update (`cargo refine upgrade`): download the release archive for
//! the running target + `SHA256SUMS`, verify the archive hash BEFORE anything
//! is extracted, extract with the system `tar` (bsdtar reads the Windows zip),
//! then backup-swap the running binary. Refuses — loudly, before touching
//! disk — a dev build (the exe lives under a cargo `target/` dir: cargo owns
//! that file), an unknown target triple (never guessed), a checksum mismatch,
//! and a missing `tar` (the install.sh fallback is named).
//!
//! The release surface is `.github/workflows/release.yml`'s package job:
//! `cargo-refine-${TAG}-${target}.tar.gz` (unix) / `.zip` (windows) plus a
//! `SHA256SUMS` over every dist file, published on the public dist repo
//! `gist-rs/cargo-refine`. `update` (the T1.6 corpus lease refresh) is a
//! different, pre-existing command — this module is `upgrade`, not `update`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The public distribution repo (Proposal 003 / Plan 105 — NEVER this source
/// repo). The releases API + the releases page are both derived from it.
pub const DIST_REPO: &str = "gist-rs/cargo-refine";
const RELEASES_API: &str = "https://api.github.com/repos/gist-rs/cargo-refine/releases/latest";

const CLIENT_TIMEOUT: Duration = Duration::from_secs(4);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// The alert re-check window after a SUCCESSFUL check (Issue 153 T1).
const OK_WINDOW_SECS: i64 = 12 * 3600;
/// The retry window after a FAILED check — a transient network failure must
/// not hide an alert for a full day.
const FAIL_WINDOW_SECS: i64 = 45 * 60;
/// The auto-corpus-refresh bookkeeping window (Issue 153 T4) — corpus epochs
/// move faster than releases, but a startup check per process is still wrong.
pub const AUTO_CORPUS_WINDOW_SECS: i64 = 12 * 3600;

/// The short-detail cap — "short detail" is the owner's words; the release
/// title (or first body line) past this is truncated with an ellipsis.
const DETAIL_CAP_CHARS: usize = 100;

// ── the release surface ─────────────────────────────────────────────────────

/// One GitHub release, narrowed to the fields this module reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The tag, verbatim (`v0.3.0`).
    pub tag: String,
    /// The tag without the leading `v` — the semver-comparable form.
    pub version: String,
    /// The short detail: the release title, else the first body line.
    pub detail: Option<String>,
    /// The release page URL (printed by the alert).
    pub url: String,
    /// `(asset name, browser_download_url)` pairs, verbatim from the API.
    pub assets: Vec<(String, String)>,
}

/// Every way this module can fail. All of them are fail-soft at the call
/// sites; the variants exist so the log line can say WHICH leg failed.
#[derive(Debug)]
pub enum UpdateError {
    /// The HTTP leg failed (transport, timeout, non-200).
    Http(String),
    /// The body was not the JSON shape this module parses.
    Parse(String),
    /// The running target triple has no release asset — refused, never guessed.
    NoAsset(String),
    /// The download/checksum/extraction/swap legs of [`upgrade`].
    Verify(String),
    /// The local filesystem legs (cache write, temp dir, rename).
    Io(std::io::Error),
    /// Not an error at the `upgrade` command — the installed build IS the
    /// latest release. Carries the version for the "already current" line.
    UpToDate(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "http: {e}"),
            Self::Parse(e) => write!(f, "parse: {e}"),
            Self::NoAsset(e) => write!(f, "asset: {e}"),
            Self::Verify(e) => write!(f, "verify: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::UpToDate(v) => write!(f, "up to date: {v}"),
        }
    }
}

/// Parse the `/releases/latest` body. `serde` with per-field defaults — a
/// release without a body or assets still parses; a body without `tag_name`
/// does not (there is nothing to compare against).
///
/// # Errors
///
/// [`UpdateError::Parse`] when the body is not the expected JSON shape.
pub fn parse_release_json(body: &str) -> Result<Release, UpdateError> {
    #[derive(serde::Deserialize)]
    struct Asset {
        #[serde(default)]
        name: String,
        #[serde(default)]
        browser_download_url: String,
    }
    #[derive(serde::Deserialize)]
    struct Raw {
        tag_name: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        html_url: Option<String>,
        #[serde(default)]
        assets: Vec<Asset>,
    }
    let raw: Raw = serde_json::from_str(body)
        .map_err(|e| UpdateError::Parse(format!("releases/latest: {e}")))?;
    let version = raw.tag_name.trim_start_matches('v').to_string();
    if version.is_empty() {
        return Err(UpdateError::Parse("empty tag_name".to_string()));
    }
    let detail = raw
        .name
        .filter(|n| !n.trim().is_empty())
        .map(|n| short_detail(&n))
        .or_else(|| raw.body.as_deref().map(short_detail));
    Ok(Release {
        tag: raw.tag_name,
        version,
        detail,
        url: raw
            .html_url
            .unwrap_or_else(|| format!("https://github.com/{DIST_REPO}/releases")),
        assets: raw
            .assets
            .into_iter()
            .filter(|a| !a.name.is_empty() && !a.browser_download_url.is_empty())
            .map(|a| (a.name, a.browser_download_url))
            .collect(),
    })
}

/// The short detail: one line, truncated, newlines flattened — the alert is
/// ONE line and stays one.
pub fn short_detail(text: &str) -> String {
    let first = text.lines().map(str::trim).find(|l| !l.is_empty());
    let mut line = first.unwrap_or_default().to_string();
    // Markdown heading + release-note markers are noise in a CLI alert.
    for prefix in ["## ", "### ", "- ", "* "] {
        if let Some(stripped) = line.strip_prefix(prefix) {
            line = stripped.trim_start().to_string();
        }
    }
    if line.chars().count() > DETAIL_CAP_CHARS {
        line = line.chars().take(DETAIL_CAP_CHARS - 1).collect::<String>() + "…";
    }
    line
}

/// Numeric semver compare — `0.10.0 > 0.9.9`; any side that does not fully
/// parse as dot-separated integers is UNORDERED (`false`, the silent
/// direction: a weird tag must never nag a working install).
#[must_use]
pub fn version_newer(latest: &str, installed: &str) -> bool {
    let parse =
        |v: &str| -> Option<Vec<u64>> { v.split('.').map(|p| p.parse::<u64>().ok()).collect() };
    match (parse(latest), parse(installed)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

// ── the cache ───────────────────────────────────────────────────────────────

/// What the last check learned. Written on EVERY attempt (ok or failed) so a
/// failure backs off too; `latest` only survives a successful parse.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateCache {
    pub checked_at_unix: i64,
    pub ok: bool,
    #[serde(default)]
    pub latest: Option<CachedRelease>,
}

/// The cached subset of [`Release`] — enough to alert without the network.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CachedRelease {
    pub version: String,
    #[serde(default)]
    pub detail: Option<String>,
    pub url: String,
    pub tag: String,
}

impl CachedRelease {
    fn from_release(r: &Release) -> Self {
        Self {
            version: r.version.clone(),
            detail: r.detail.clone(),
            url: r.url.clone(),
            tag: r.tag.clone(),
        }
    }
}

#[must_use]
pub fn cache_path(root: &Path) -> PathBuf {
    root.join(".refine").join("update_check.json")
}

#[must_use]
pub fn load_cache(path: &Path) -> Option<UpdateCache> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Persist the cache (temp + rename — a reader never sees a torn file).
///
/// # Errors
///
/// [`UpdateError::Io`] when the directory cannot be created or the file
/// cannot be written/renamed; a failed cache write is always ignorable at
/// the call sites.
pub fn store_cache(path: &Path, cache: &UpdateCache) -> Result<(), UpdateError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(UpdateError::Io)?;
    }
    // Temp + rename — a reader never sees a torn cache file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(cache).map_err(|e| UpdateError::Parse(e.to_string()))?)
        .map_err(UpdateError::Io)?;
    std::fs::rename(&tmp, path).map_err(UpdateError::Io)
}

/// What to do, decided purely from (cache, now, installed) — the testable
/// heart of the alert. Fetch windows: ok → [`OK_WINDOW_SECS`], failed →
/// [`FAIL_WINDOW_SECS`].
#[derive(Debug)]
pub enum Plan {
    /// The cache already knows a newer release — alert from it, no network.
    Alert(CachedRelease),
    /// The cache is stale/absent — fetch, then decide.
    Fetch,
    /// Nothing to do (fresh cache, no newer release, or an unordered tag).
    Silent,
}

#[must_use]
pub fn decide(cache: Option<&UpdateCache>, now: i64, installed: &str) -> Plan {
    match cache {
        None => Plan::Fetch,
        Some(c) => {
            let window = if c.ok { OK_WINDOW_SECS } else { FAIL_WINDOW_SECS };
            if now - c.checked_at_unix < window {
                match &c.latest {
                    Some(l) if version_newer(&l.version, installed) => Plan::Alert(l.clone()),
                    _ => Plan::Silent,
                }
            } else {
                Plan::Fetch
            }
        }
    }
}

/// The alert line, or `None` when the installed build is current (or the tag
/// is unordered — see [`version_newer`]).
#[must_use]
pub fn alert_line(latest: &CachedRelease, installed: &str) -> Option<String> {
    if !version_newer(&latest.version, installed) {
        return None;
    }
    let detail = latest
        .detail
        .as_deref()
        .map(|d| format!(" — {d}"))
        .unwrap_or_default();
    Some(format!(
        "update available: cargo-refine {} (installed {}){}\n  {}",
        latest.version, installed, detail, latest.url
    ))
}

// ── the fetcher (the one network seam) ──────────────────────────────────────

/// The release fetch, abstracted so every decision path is testable without
/// the network. The real impl is [`GitHubLatest`].
pub trait FetchLatest {
    /// # Errors
    ///
    /// [`UpdateError::Http`] on any transport/status failure,
    /// [`UpdateError::Parse`] when the body is not the expected shape.
    fn fetch(&self) -> Result<Release, UpdateError>;
}

/// The real fetcher: GET `/releases/latest` with a User-Agent (the GitHub API
/// refuses requests without one).
pub struct GitHubLatest {
    agent: ureq::Agent,
}

impl GitHubLatest {
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(CLIENT_TIMEOUT)
                .timeout_connect(CONNECT_TIMEOUT)
                .build(),
        }
    }
}

impl Default for GitHubLatest {
    fn default() -> Self {
        Self::new()
    }
}

impl FetchLatest for GitHubLatest {
    fn fetch(&self) -> Result<Release, UpdateError> {
        let installed = env!("CARGO_PKG_VERSION");
        let resp = self
            .agent
            .get(RELEASES_API)
            .set("User-Agent", &format!("cargo-refine/{installed}"))
            .set("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| UpdateError::Http(e.to_string()))?;
        let status = resp.status();
        let mut body = String::new();
        resp.into_reader()
            .take(4 * 1024 * 1024)
            .read_to_string(&mut body)
            .map_err(|e| UpdateError::Http(format!("read: {e}")))?;
        if status != 200 {
            return Err(UpdateError::Http(format!("status {status}")));
        }
        parse_release_json(&body)
    }
}

// ── T1: the alert entry point ───────────────────────────────────────────────

/// The env gate: `RIIR_REFINE_UPDATE_CHECK=0` opts out; a `CI` env skips.
/// Anything else (including unset) is on — the owner asked for the alert.
#[must_use]
pub fn alert_enabled() -> bool {
    if std::env::var("CI").is_ok_and(|v| !v.is_empty()) {
        return false;
    }
    std::env::var("RIIR_REFINE_UPDATE_CHECK").as_deref() != Ok("0")
}

/// The hands-free upgrade gate (T2): unset/anything-but-1 = off. A binary
/// that replaces itself is OPT-IN, full stop.
#[must_use]
pub fn auto_upgrade_enabled() -> bool {
    std::env::var("RIIR_REFINE_AUTO_UPGRADE").as_deref() == Ok("1")
}

/// Run the check and return the alert lines to print (empty = silent). Never
/// returns `Err` — every failure is a silent skip or part of the returned
/// diagnostics the caller may print. Writes the cache on every attempt.
pub fn check_for_alert(root: &Path, fetcher: &dyn FetchLatest) -> Vec<String> {
    if !alert_enabled() {
        return Vec::new();
    }
    let installed = env!("CARGO_PKG_VERSION");
    let path = cache_path(root);
    let now = unix_now();
    let plan = decide(load_cache(&path).as_ref(), now, installed);
    let latest = match plan {
        Plan::Alert(cached) => cached,
        Plan::Silent => return Vec::new(),
        Plan::Fetch => {
            let Ok(r) = fetcher.fetch() else {
                // Back off quietly — but write the attempt so a flaky network
                // does not re-timeout on every invocation.
                let _ = store_cache(
                    &path,
                    &UpdateCache {
                        checked_at_unix: unix_now(),
                        ok: false,
                        latest: None,
                    },
                );
                return Vec::new();
            };
            let cached = CachedRelease::from_release(&r);
            let _ = store_cache(
                &path,
                &UpdateCache {
                    checked_at_unix: unix_now(),
                    ok: true,
                    latest: Some(cached.clone()),
                },
            );
            if !version_newer(&cached.version, installed) {
                return Vec::new();
            }
            cached
        }
    };
    alert_line(&latest, installed)
        .map(|line| vec![line])
        .unwrap_or_default()
}

// ── T2: the verified self-update ────────────────────────────────────────────

/// The release.yml package job's five targets. The running triple must be one
/// of these — an unknown triple REFUSES rather than guessing an asset.
#[must_use]
pub fn release_target_triple() -> Option<&'static str> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Some("aarch64-apple-darwin")
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Some("x86_64-apple-darwin")
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
    {
        Some("x86_64-unknown-linux-musl")
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
    {
        Some("aarch64-unknown-linux-musl")
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "gnu"))]
    {
        Some("x86_64-pc-windows-gnu")
    }
    // Any other triple (msvc windows, gnu linux, …) has no release asset —
    // the release workflow does not build one, so refusing is the TRUTH.
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64", target_env = "musl"),
        all(target_os = "linux", target_arch = "aarch64", target_env = "musl"),
        all(target_os = "windows", target_arch = "x86_64", target_env = "gnu"),
    )))]
    {
        None
    }
}

/// The archive asset name, exactly as the package job writes it.
#[must_use]
pub fn asset_name(tag: &str, triple: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("cargo-refine-{tag}-{triple}.zip")
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!("cargo-refine-{tag}-{triple}.tar.gz")
    }
}

/// What [`upgrade`] did — the caller prints the human summary from this.
#[derive(Debug)]
pub struct UpgradeReport {
    pub from_version: String,
    pub to_version: String,
    pub new_exe: PathBuf,
    /// The old binary's backup path (the rollback story).
    pub backup: PathBuf,
    /// When the swapped-out destination was a SYMLINK (the developer
    /// rebuild-convention link), its target — the backup rename moves the
    /// link itself, so the convention survives at [`Self::backup`] while
    /// the destination now serves release bytes. `None` for a plain-file
    /// destination (normal users; no notice).
    pub replaced_symlink_target: Option<PathBuf>,
}

impl UpgradeReport {
    /// Issue 164: the one-line courtesy notice for a swap that moved a
    /// developer symlink aside — the rebuild link no longer propagates
    /// until restored, and the restore command differs by platform
    /// (non-elevated Windows cannot re-link at all).
    #[must_use]
    pub fn symlink_notice(&self) -> Option<String> {
        let target = self.replaced_symlink_target.as_ref()?;
        Some(if cfg!(windows) {
            format!(
                "note: the destination was a developer symlink -> {} (preserved at {}); \
                 non-elevated Windows cannot re-link — re-copy the link target over {} \
                 after each rebuild (or mklink elevated)",
                target.display(),
                self.backup.display(),
                self.new_exe.display()
            )
        } else {
            format!(
                "note: the destination was a developer symlink -> {} (preserved at {}); \
                 restore the rebuild link: ln -sf '{}' '{}'",
                target.display(),
                self.backup.display(),
                target.display(),
                self.new_exe.display()
            )
        })
    }
}

/// The download seam (unit-testable: the checksum/extract/swap legs run
/// against local files).
pub trait Download {
    /// # Errors
    ///
    /// [`UpdateError::Http`] on transport/status failure, [`UpdateError::Io`]
    /// when the destination file cannot be created or written.
    fn download_to(&self, url: &str, dest: &Path) -> Result<(), UpdateError>;
}

pub struct HttpDownload {
    agent: ureq::Agent,
}

impl HttpDownload {
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(120))
                .timeout_connect(CONNECT_TIMEOUT)
                .build(),
        }
    }
}

impl Default for HttpDownload {
    fn default() -> Self {
        Self::new()
    }
}

impl Download for HttpDownload {
    fn download_to(&self, url: &str, dest: &Path) -> Result<(), UpdateError> {
        let installed = env!("CARGO_PKG_VERSION");
        let resp = self
            .agent
            .get(url)
            .set("User-Agent", &format!("cargo-refine/{installed}"))
            .call()
            .map_err(|e| UpdateError::Http(e.to_string()))?;
        if resp.status() != 200 {
            return Err(UpdateError::Http(format!("status {} for {url}", resp.status())));
        }
        let mut file = std::fs::File::create(dest).map_err(UpdateError::Io)?;
        std::io::copy(&mut resp.into_reader().take(512 * 1024 * 1024), &mut file)
            .map_err(std::io::Error::other)
            .map_err(UpdateError::Io)?;
        file.sync_all().map_err(UpdateError::Io)
    }
}

/// Verify `want_hex == sha256(bytes)` in constant-ish time (branch on the
/// whole comparison, not per-byte early exit).
fn sha256_matches(bytes: &[u8], want_hex: &str) -> bool {
    use sha2::Digest;
    use std::fmt::Write as _;
    let digest = sha2::Sha256::digest(bytes);
    let mut got = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(got, "{byte:02x}");
    }
    got.eq_ignore_ascii_case(want_hex.trim())
}

/// Pull the archive's hash line out of a `sha256sum`-format SHA256SUMS body
/// (`<hex>  <filename>` — two spaces). Missing line = a loud mismatch, never
/// a skip (a sums file that does not name our asset is a packaging bug).
fn sums_hash_for(sums_body: &str, asset: &str) -> Option<String> {
    for line in sums_body.lines() {
        let mut parts = line.splitn(2, char::is_whitespace);
        let hex = parts.next()?.trim();
        let name = parts.next()?.trim();
        // sha256sum emits "hex  name" (two spaces); splitn(2, ws) leaves the
        // leading whitespace on the name — trim handles both spellings.
        let name = name.trim_start();
        if name == asset {
            return Some(hex.to_string());
        }
    }
    None
}

/// Refuse a DEV build: an exe under a cargo `target/` dir is cargo's artifact;
/// swapping it corrupts the incremental state and lies about `cargo build`.
#[must_use]
pub fn is_dev_build(exe: &Path) -> bool {
    exe.ancestors()
        .any(|a| a.file_name().is_some_and(|n| n == "target"))
}

/// The full upgrade: fetch → pick asset → download archive + SHA256SUMS →
/// verify → extract → backup-swap. Every refusal lands BEFORE the running
/// binary is touched.
///
/// # Errors
///
/// [`UpdateError::UpToDate`] when the installed build IS the latest (not a
/// failure at the CLI); [`UpdateError::NoAsset`] for an unknown platform or
/// a missing asset; [`UpdateError::Verify`] for the checksum/extract/swap
/// refusals; [`UpdateError::Io`] for the filesystem legs; [`UpdateError::Http`]
/// from the fetch/download legs.
pub fn upgrade(
    fetcher: &dyn FetchLatest,
    downloader: &dyn Download,
    installed: &str,
) -> Result<UpgradeReport, UpdateError> {
    let exe = std::env::current_exe().map_err(UpdateError::Io)?;
    if is_dev_build(&exe) {
        return Err(UpdateError::Verify(format!(
            "{} looks like a cargo target-dir build — upgrade it with cargo, not self-update",
            exe.display()
        )));
    }
    let triple = release_target_triple().ok_or_else(|| {
        UpdateError::NoAsset(format!(
            "no release asset for this platform ({}) — use the install script at https://github.com/{DIST_REPO}",
            std::env::consts::ARCH
        ))
    })?;
    let release = fetcher.fetch()?;
    if !version_newer(&release.version, installed) {
        return Err(UpdateError::UpToDate(release.version));
    }
    let asset = asset_name(&release.tag, triple);
    let find = |name: &str| -> Result<String, UpdateError> {
        release
            .assets
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, u)| u.clone())
            .ok_or_else(|| {
                UpdateError::NoAsset(format!(
                    "release {} carries no {name} asset — packaging bug, refusing",
                    release.tag
                ))
            })
    };
    let archive_url = find(&asset)?;
    let sums_url = find("SHA256SUMS")?;

    // Everything stages beside the exe (same volume ⇒ rename is atomic).
    let exe_dir = exe.parent().ok_or_else(|| {
        UpdateError::Verify(format!("exe has no parent dir: {}", exe.display()))
    })?;
    let stage = exe_dir.join(format!(".cargo-refine-upgrade-{}", std::process::id()));
    std::fs::create_dir_all(&stage).map_err(UpdateError::Io)?;
    let staged = StagedUpgrade {
        stage: &stage,
        archive_url: &archive_url,
        sums_url: &sums_url,
        asset: &asset,
        to_version: &release.version,
    };
    let result = staged.install(downloader, &exe, installed);
    let _ = std::fs::remove_dir_all(&stage);
    result
}

/// The per-release install inputs — borrowed, so the orchestration above
/// stays short and the swap logic below stays testable against a real dir.
struct StagedUpgrade<'a> {
    stage: &'a Path,
    archive_url: &'a str,
    sums_url: &'a str,
    asset: &'a str,
    to_version: &'a str,
}

impl StagedUpgrade<'_> {
    /// Download → verify checksum → extract → chmod → backup-swap. On a
    /// failed swap the old binary is put BACK — a failed upgrade must never
    /// strand the user with no binary.
    fn install(
        &self,
        downloader: &dyn Download,
        exe: &Path,
        installed: &str,
    ) -> Result<UpgradeReport, UpdateError> {
        let archive = self.stage.join(self.asset);
        let sums = self.stage.join("SHA256SUMS");
        downloader.download_to(self.archive_url, &archive)?;
        downloader.download_to(self.sums_url, &sums)?;
        let sums_body = std::fs::read_to_string(&sums).map_err(UpdateError::Io)?;
        let want = sums_hash_for(&sums_body, self.asset).ok_or_else(|| {
            UpdateError::Verify(format!(
                "SHA256SUMS names no line for {} — refusing an unchecksummed archive",
                self.asset
            ))
        })?;
        let archive_bytes = std::fs::read(&archive).map_err(UpdateError::Io)?;
        if !sha256_matches(&archive_bytes, &want) {
            return Err(UpdateError::Verify(format!(
                "sha256 mismatch for {} — refusing (the release may be republishing; retry later)",
                self.asset
            )));
        }

        // Extract with the system tar (bsdtar reads the Windows zip too).
        let payload = payload_name();
        let status = std::process::Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(self.stage)
            .status()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    UpdateError::Verify(
                        "no `tar` on PATH — install via https://github.com/gist-rs/cargo-refine (install.sh / install.ps1) instead"
                            .to_string(),
                    )
                } else {
                    UpdateError::Verify(format!("tar spawn: {e}"))
                }
            })?;
        if !status.success() {
            return Err(UpdateError::Verify(format!("tar exited {status}")));
        }
        let new_exe = self.stage.join(payload);
        if !new_exe.is_file() {
            return Err(UpdateError::Verify(format!(
                "archive extracted but no {payload} inside — packaging bug, refusing"
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&new_exe, std::fs::Permissions::from_mode(0o755))
                .map_err(UpdateError::Io)?;
        }

        // Backup-swap: the old binary stays as .bak-<version> for rollback.
        // Issue 164: rename moves a SYMLINK itself (never its target), so a
        // developer box's rebuild link survives at `backup` — capture the
        // target first and let the caller print the restore notice.
        let replaced_symlink_target = std::fs::read_link(exe).ok();
        let backup = exe.with_file_name(format!(
            "{}.bak-{}",
            exe.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cargo-refine"),
            installed
        ));
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(exe, &backup).map_err(UpdateError::Io)?;
        if let Err(e) = std::fs::rename(&new_exe, exe) {
            // Put the old binary BACK — a failed swap must not strand the
            // user with no binary.
            let _ = std::fs::rename(&backup, exe);
            return Err(UpdateError::Verify(format!("swap: {e}")));
        }
        Ok(UpgradeReport {
            from_version: installed.to_string(),
            to_version: self.to_version.to_string(),
            new_exe: exe.to_path_buf(),
            backup,
            replaced_symlink_target,
        })
    }
}

/// The binary's name inside the archive (the package job stages it bare).
fn payload_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "cargo-refine.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "cargo-refine"
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "update_check"))]
mod tests {
    use super::*;

    const INSTALLED: &str = "0.2.1";

    fn release(version: &str, detail: Option<&str>) -> CachedRelease {
        CachedRelease {
            version: version.to_string(),
            detail: detail.map(str::to_string),
            url: "https://github.com/gist-rs/cargo-refine/releases/tag/v1.0.0".to_string(),
            tag: format!("v{version}"),
        }
    }

    // ── version compare ──
    #[test]
    fn newer_semver_matrix() {
        assert!(version_newer("0.3.0", "0.2.1"));
        assert!(version_newer("0.10.0", "0.9.9"), "numeric, not lexicographic");
        assert!(version_newer("1.0.0", "0.99.99"));
        assert!(!version_newer("0.2.1", "0.2.1"));
        assert!(!version_newer("0.2.0", "0.2.1"));
        // Ragged tags are UNORDERED — silent, never a nag.
        assert!(!version_newer("v1-rc1", "0.2.1"));
        assert!(!version_newer("garbage", "0.2.1"));
        assert!(!version_newer("0.3.0", "garbage"));
    }

    // ── release JSON parse ──
    #[test]
    fn parse_extracts_tag_title_assets_and_prefers_the_title() {
        let body = r###"{
            "tag_name": "v0.3.0",
            "name": "v0.3.0 — corpus: +12 rust_perf rules",
            "body": "## Highlights\n- faster drafter\n",
            "html_url": "https://github.com/gist-rs/cargo-refine/releases/tag/v0.3.0",
            "assets": [
                {"name": "cargo-refine-v0.3.0-aarch64-apple-darwin.tar.gz",
                 "browser_download_url": "https://example/a.tar.gz"},
                {"name": "SHA256SUMS", "browser_download_url": "https://example/sums"}
            ]
        }"###;
        let r = parse_release_json(body).expect("parses");
        assert_eq!(r.version, "0.3.0");
        assert_eq!(r.detail.as_deref(), Some("v0.3.0 — corpus: +12 rust_perf rules"));
        assert_eq!(r.assets.len(), 2);
        assert_eq!(r.assets[1].0, "SHA256SUMS");
    }

    #[test]
    fn parse_falls_back_to_the_first_body_line_and_truncates() {
        let body = r#"{"tag_name": "v1.2.3", "name": "", "body": "first line here\nsecond line\n"}"#;
        let r = parse_release_json(body).expect("parses");
        assert_eq!(r.detail.as_deref(), Some("first line here"));

        let long = "x".repeat(300);
        let body = format!(r#"{{"tag_name": "v1.0.0", "body": "{long}"}}"#);
        let r = parse_release_json(&body).expect("parses");
        let detail = r.detail.expect("detail present");
        assert!(detail.chars().count() <= DETAIL_CAP_CHARS, "{detail}");
        assert!(detail.ends_with('…'));
    }

    #[test]
    fn parse_refuses_a_body_without_a_tag() {
        assert!(parse_release_json("{\"name\": \"no tag\"}").is_err());
        assert!(parse_release_json("not json").is_err());
    }

    // ── the decide matrix ──
    #[test]
    fn decide_fresh_ok_cache_alerts_without_the_network() {
        let cache = UpdateCache {
            checked_at_unix: 1_000_000,
            ok: true,
            latest: Some(release("9.0.0", None)),
        };
        match decide(Some(&cache), 1_000_000 + 3600, INSTALLED) {
            Plan::Alert(r) => assert_eq!(r.version, "9.0.0"),
            other => panic!("expected Alert, got {other:?}"),
        }
    }

    #[test]
    fn decide_windows_split_ok_from_failed() {
        let ok_cache = UpdateCache { checked_at_unix: 1_000_000, ok: true, latest: None };
        let fail_cache = UpdateCache { checked_at_unix: 1_000_000, ok: false, latest: None };
        // A failed check re-fetches well inside the ok window…
        assert!(matches!(
            decide(Some(&fail_cache), 1_000_000 + 3600, INSTALLED),
            Plan::Fetch
        ));
        // …and an ok cache stays silent across the same span…
        assert!(matches!(
            decide(Some(&ok_cache), 1_000_000 + 3600, INSTALLED),
            Plan::Silent
        ));
        // …but BOTH go stale past their own window.
        assert!(matches!(decide(Some(&ok_cache), 1_000_000 + OK_WINDOW_SECS, INSTALLED), Plan::Fetch));
        assert!(matches!(
            decide(Some(&fail_cache), 1_000_000 + FAIL_WINDOW_SECS, INSTALLED),
            Plan::Fetch
        ));
    }

    #[test]
    fn decide_fresh_cache_with_current_version_is_silent() {
        let cache = UpdateCache {
            checked_at_unix: 1_000_000,
            ok: true,
            latest: Some(release(INSTALLED, None)),
        };
        assert!(matches!(decide(Some(&cache), 1_000_000 + 10, INSTALLED), Plan::Silent));
        assert!(matches!(decide(None, 0, INSTALLED), Plan::Fetch));
    }

    // ── the alert text ──
    #[test]
    fn alert_line_carries_versions_detail_and_url() {
        let line = alert_line(&release("0.3.0", Some("corpus: +12 rust_perf rules")), INSTALLED)
            .expect("newer alerts");
        assert!(line.contains("cargo-refine 0.3.0 (installed 0.2.1)"), "{line}");
        assert!(line.contains("corpus: +12 rust_perf rules"), "{line}");
        assert!(line.contains("https://github.com/gist-rs/cargo-refine"), "{line}");
        assert!(alert_line(&release(INSTALLED, None), INSTALLED).is_none());
        assert!(alert_line(&release("0.1.0", None), INSTALLED).is_none());
    }

    // ── asset naming + triples ──
    #[test]
    fn asset_name_matches_the_release_workflow() {
        assert_eq!(
            asset_name("v0.3.0", "aarch64-apple-darwin"),
            "cargo-refine-v0.3.0-aarch64-apple-darwin.tar.gz"
        );
    }

    // ── issue 164: the developer-symlink courtesy notice ──
    #[test]
    fn symlink_notice_names_target_backup_and_restore() {
        let report = UpgradeReport {
            from_version: "0.2.1".into(),
            to_version: "0.3.0".into(),
            new_exe: PathBuf::from("/home/dev/.cargo/bin/cargo-refine"),
            backup: PathBuf::from("/home/dev/.cargo/bin/cargo-refine.bak-0.2.1"),
            replaced_symlink_target: Some(PathBuf::from(
                "/git/riir-refine/target/release/cargo-refine",
            )),
        };
        let notice = report.symlink_notice().expect("a symlink swap notices");
        assert!(notice.contains("developer symlink"), "{notice}");
        assert!(notice.contains("target/release/cargo-refine"), "{notice}");
        assert!(notice.contains(".bak-0.2.1"), "{notice}");
        #[cfg(unix)]
        assert!(
            notice.contains("ln -sf"),
            "unix names the restore command: {notice}"
        );
        #[cfg(windows)]
        assert!(
            notice.contains("re-copy"),
            "windows names the re-copy remedy: {notice}"
        );

        // A plain-file destination (the normal user) prints nothing.
        let plain = UpgradeReport {
            replaced_symlink_target: None,
            ..report
        };
        assert!(plain.symlink_notice().is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn this_macs_triple_is_a_release_target() {
        assert!(release_target_triple().is_some_and(|t| t.ends_with("apple-darwin")));
    }

    // ── SHA256SUMS parsing + verify ──
    #[test]
    fn sums_hash_finds_the_asset_line() {
        let body = "abc123  cargo-refine-v1.0.0-x86_64-unknown-linux-musl.tar.gz\n\
                    def456  SHA256SUMS\n";
        assert_eq!(
            sums_hash_for(body, "cargo-refine-v1.0.0-x86_64-unknown-linux-musl.tar.gz")
                .as_deref(),
            Some("abc123")
        );
        assert_eq!(sums_hash_for(body, "SHA256SUMS").as_deref(), Some("def456"));
        assert!(sums_hash_for(body, "missing.tar.gz").is_none());
    }

    #[test]
    fn sha256_verify_good_and_bad() {
        assert!(sha256_matches(
            b"hello",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        ));
        assert!(sha256_matches(
            b"hello",
            "2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824"
        ));
        assert!(!sha256_matches(b"hello", "0000"));
        assert!(!sha256_matches(b"hellO", "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"));
    }

    // ── the dev-build refusal ──
    #[test]
    fn dev_build_detection() {
        assert!(is_dev_build(Path::new("/repo/target/release/cargo-refine")));
        assert!(is_dev_build(Path::new("/repo/target/dist/cargo-refine")));
        assert!(!is_dev_build(Path::new("/usr/local/bin/cargo-refine")));
        assert!(!is_dev_build(Path::new(
            "/Users/x/.cargo/bin/cargo-refine"
        )));
    }

    // ── the cache round trip ──
    #[test]
    fn cache_round_trips_through_a_pid_scoped_temp_dir() {
        let dir = std::env::temp_dir().join(format!("refine_updchk_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("update_check.json");
        let cache = UpdateCache {
            checked_at_unix: 42,
            ok: true,
            latest: Some(release("9.9.9", Some("d"))),
        };
        store_cache(&path, &cache).unwrap();
        let loaded = load_cache(&path).unwrap();
        assert_eq!(loaded, cache);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── the short detail ──
    #[test]
    fn short_detail_strips_markdown_and_flattens() {
        assert_eq!(short_detail("## Highlights\n- fast thing"), "Highlights");
        assert_eq!(short_detail("plain"), "plain");
        assert_eq!(short_detail("   \n  indented  "), "indented");
        assert_eq!(short_detail(""), "");
    }
}
