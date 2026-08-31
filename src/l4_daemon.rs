//! `DaemonL4` — an [`L4Fallback`] backed by the riir-train eval daemon.
//!
//! The adapter is a 27B ternary model with a 135 s load, so the only viable
//! transport is load-once/serve-many: riir-train `80601f19` added `--serve` to
//! `bonsai_clippy_l4_eval`, measured at ~193 s for the first fix and ~24 s for
//! each one after (against ~193 s *every* time for a subprocess-per-fix shape,
//! which is worse than the CPU baseline that already failed G2).
//!
//! This module is the client half: it owns the child process and speaks the
//! protocol. std-only — no model dependency reaches riir-clippy.
//!
//! ## Protocol (must match `bonsai_clippy_l4_eval --serve`)
//!
//! | direction | line |
//! |---|---|
//! | daemon → us | `<<<READY>>>` once the model is resident |
//! | us → daemon | one request per line, real newlines escaped as `\n` |
//! | daemon → us | `<<<FIX n>>>`, the fix, `<<<END n>>>` |
//! | us → daemon | `<<<QUIT>>>` to stop (also stops on EOF) |
//!
//! ## Two things that are easy to get wrong
//!
//! **The prompt must not mention the lint.** The seam hands us a `rule_id`, and
//! the obvious move is to put it in the prompt. That would be off-distribution:
//! all 1092 rows of the v2edit training corpus share one lint-agnostic
//! instruction line ("Fix all clippy lints in this Rust code…"), so the adapter
//! never saw a lint name. `rule_id` is used for logging only.
//!
//! **Edit-only output is a fragment, and the seam gates on the L2 pruner.** The
//! adapter is trained to emit only the changed lines, which is frequently not
//! parseable on its own — the pruner's `fn _wrap() { … }` fallback rescues
//! statement-shaped fragments but not ones with unbalanced delimiters. Measured
//! on the two real daemon responses captured 2026-08-17 (see `tests`): one
//! parses, one does not. So a G1-passing adapter will still have a fraction of
//! its answers discarded by invariant 3, and that fraction is a property of the
//! response contract, not a bug in this client.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use crate::l4_fallback::L4Fallback;

/// The instruction line every training row shares. Reproduced verbatim; a
/// reworded prompt is a different distribution than the one trained on.
///
/// `pub` since Issue 040: the CF-Workers-AI re-gate harness builds its arm-A
/// prompt from the same line so both arms share one instruction contract.
pub const TRAINED_INSTRUCTION: &str =
    "Fix all clippy lints in this Rust code. Output ONLY the corrected lines \
     that differ from the input, no explanation:";

/// An [`L4Fallback`] that talks to a long-lived `bonsai_clippy_l4_eval --serve`
/// process.
pub struct DaemonL4 {
    child: Child,
    /// `Option` so [`Drop`] can CLOSE the pipe before waiting. Writing
    /// `<<<QUIT>>>` is not enough on its own: a daemon that keeps reading sees
    /// no EOF while we still hold the write end, so `wait()` blocks forever.
    /// This deadlocked the test suite before the handle was made droppable.
    stdin: Option<ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
    name: String,
    /// Set once the child is known to be unusable. Every later call declines
    /// immediately rather than blocking on a pipe nobody will write to.
    dead: bool,
}

impl DaemonL4 {
    /// Spawn the daemon and block until it signals `<<<READY>>>`.
    ///
    /// # Errors
    ///
    /// - the binary cannot be spawned (bad path, not executable);
    /// - its stdio pipes cannot be captured;
    /// - it exits during load without signalling ready — a 27B load failure
    ///   (missing GGUF, bad checkpoint) must surface here, not as a mysterious
    ///   decline on the first heal.
    ///
    /// Never panics: the trait contract forbids a fallback from taking down a
    /// heal that would otherwise have returned a modelless answer, and that is
    /// only credible if construction cannot panic either.
    pub fn spawn(
        binary: &str,
        gguf: &str,
        lora: &str,
        max_new: usize,
    ) -> Result<Self, std::io::Error> {
        let mut child = Command::new(binary)
            .args(["--gguf", gguf, "--lora", lora, "--serve", "--max-new"])
            .arg(max_new.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let (Some(stdin), Some(child_out)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::other("could not capture L4 daemon stdio"));
        };
        let mut stdout = BufReader::new(child_out);

        // The daemon prints its banner and load progress before READY. Consume
        // until the handshake rather than assuming a line count — the banner
        // has changed twice already.
        let mut line = String::new();
        loop {
            line.clear();
            let n = stdout.read_line(&mut line)?;
            if n == 0 {
                let _ = child.kill();
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "L4 daemon exited during load without signalling READY \
                     (check its stderr: missing GGUF or bad checkpoint)",
                ));
            }
            if line.trim_end() == "<<<READY>>>" {
                break;
            }
        }

        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout,
            name: format!("daemon-l4({})", short(lora)),
            dead: false,
        })
    }

    /// One request/response round trip with a CALLER-BUILT prompt (Issue 040
    /// arm C pre-pass: the harness composes instruction + module + modelless
    /// candidate + few-shots and needs the daemon transport without the
    /// trait's fixed prompt shape). Same protocol + failure contract as
    /// [`L4Fallback::generate_fix`].
    pub fn request_prompt(&mut self, prompt: &str) -> Option<String> {
        self.request(prompt)
    }

    /// One request/response round trip. `None` on any protocol or process
    /// failure — declining is always safe for the caller.
    fn request(&mut self, prompt: &str) -> Option<String> {
        if self.dead {
            return None;
        }
        // Real newlines would end the request early: the protocol is
        // line-oriented, so the prompt is escaped and the daemon un-escapes.
        let escaped = prompt.replace('\n', "\\n");
        let Some(stdin) = self.stdin.as_mut() else {
            self.dead = true;
            return None;
        };
        if writeln!(stdin, "{escaped}").is_err() || stdin.flush().is_err() {
            self.dead = true;
            return None;
        }

        let mut body = String::new();
        let mut in_fix = false;
        let mut line = String::new();
        loop {
            line.clear();
            match self.stdout.read_line(&mut line) {
                Ok(0) => {
                    // EOF mid-request: the daemon died. Mark dead so the next
                    // call is instant instead of another blocked read.
                    self.dead = true;
                    return None;
                }
                Ok(_) => {}
                Err(_) => {
                    self.dead = true;
                    return None;
                }
            }
            let t = line.trim_end_matches(['\n', '\r']);
            if t.starts_with("<<<FIX ") {
                in_fix = true;
                continue;
            }
            if t.starts_with("<<<END ") {
                break;
            }
            if in_fix {
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(t);
            }
        }

        // An empty frame is the daemon's "I could not serve this" answer.
        if body.trim().is_empty() { None } else { Some(body) }
    }
}

impl L4Fallback for DaemonL4 {
    fn generate_fix(&mut self, _rule_id: &str, buggy_code: &str) -> Option<String> {
        // `_rule_id` is deliberately unused in the prompt — see the module doc.
        self.request(&format!("{TRAINED_INSTRUCTION}\n\n{buggy_code}"))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for DaemonL4 {
    fn drop(&mut self) {
        // Order matters, and getting it wrong deadlocks:
        //   1. ask politely,
        //   2. CLOSE the write end so a daemon that ignores <<<QUIT>>> still
        //      sees EOF — while we hold this handle it never will,
        //   3. only then wait.
        // A 27B process left running holds ~7 GB, so this must be reliable.
        if let Some(mut stdin) = self.stdin.take() {
            let _ = writeln!(stdin, "<<<QUIT>>>");
            let _ = stdin.flush();
            // `stdin` drops here, closing the pipe.
        }
        // Belt and braces: a wedged child must not outlive us holding 7 GB.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Last path component, for a readable adapter name in logs.
fn short(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    // The fake-daemon suite is `#[cfg(unix)]`-gated below (Windows cannot
    // exec `#!` scripts — error 193), so the helper + `ECHO_DAEMON` are
    // dead code on non-unix hosts.
    #![cfg_attr(not(unix), allow(dead_code))]

    #[cfg(unix)]
    use super::{DaemonL4, L4Fallback};
    #[cfg(unix)]
    use std::io::Write;

    /// A fake daemon: speaks the protocol, needs no model. Lets the client's
    /// framing, escaping and failure handling be tested in milliseconds.
    ///
    /// The suite is `#[cfg(unix)]`-gated: the scripts are `#!/bin/sh` programs,
    /// and `Command::spawn` on Windows has no shebang mechanism (error 193,
    /// "%1 is not a valid Win32 application"). The protocol contracts these
    /// tests pin (banner skip, escaping, EOF-decline) are Windows-independent;
    /// they rejoin the suite on POSIX hosts.
    #[cfg(unix)]
    fn fake_daemon(script: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("l4d_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("fake_{:x}.sh", script.len()));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(script.as_bytes()).unwrap();
        drop(f);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        p
    }

    const ECHO_DAEMON: &str = r#"#!/bin/sh
echo "banner line the client must skip"
echo "<<<READY>>>"
n=0
while IFS= read -r line; do
  [ "$line" = "<<<QUIT>>>" ] && exit 0
  n=$((n+1))
  echo "<<<FIX $n>>>"
  # printf, NOT echo: POSIX echo expands backslash escapes, so `echo "$line"`
  # would turn the client's literal \n back into real newlines and the test
  # would "prove" the client failed to escape. The daemon under test uses
  # printf for the same reason.
  printf '%s\n' "$line"
  echo "<<<END $n>>>"
done
"#;

    #[cfg(unix)]
    #[test]
    fn handshake_skips_the_banner_and_reaches_ready() {
        let bin = fake_daemon(ECHO_DAEMON);
        let d = DaemonL4::spawn(bin.to_str().unwrap(), "g", "/tmp/ckpt.bin", 96);
        assert!(d.is_ok(), "spawn should clear the banner and see READY");
        assert_eq!(d.unwrap().name(), "daemon-l4(ckpt.bin)");
    }

    #[cfg(unix)]
    #[test]
    fn prompt_uses_the_trained_instruction_and_omits_the_lint_name() {
        let bin = fake_daemon(ECHO_DAEMON);
        let mut d = DaemonL4::spawn(bin.to_str().unwrap(), "g", "c.bin", 96).unwrap();
        let echoed = d.generate_fix("needless_return", "fn g() -> i32 {\n    return 7;\n}").unwrap();

        assert!(
            echoed.starts_with("Fix all clippy lints in this Rust code."),
            "must send the trained instruction verbatim, got: {echoed}"
        );
        assert!(
            !echoed.contains("needless_return"),
            "the lint name must NOT reach the prompt — all 1092 training rows \
             are lint-agnostic, so including it is off-distribution: {echoed}"
        );
        assert!(echoed.contains("\\n"), "newlines must be escaped for the line protocol");
    }

    #[cfg(unix)]
    #[test]
    fn dead_daemon_declines_instead_of_blocking() {
        let bin = fake_daemon("#!/bin/sh\necho \"<<<READY>>>\"\nexit 0\n");
        let mut d = DaemonL4::spawn(bin.to_str().unwrap(), "g", "c.bin", 96).unwrap();
        assert!(d.generate_fix("x", "code").is_none(), "EOF must decline, not hang");
        assert!(d.generate_fix("x", "code").is_none(), "second call must decline immediately");
    }

    #[cfg(unix)]
    #[test]
    fn daemon_that_dies_during_load_is_an_error_not_a_silent_decline() {
        let bin = fake_daemon("#!/bin/sh\necho loading\nexit 1\n");
        let e = DaemonL4::spawn(bin.to_str().unwrap(), "g", "c.bin", 96);
        assert!(e.is_err(), "a load failure must surface at spawn");
    }

    #[cfg(unix)]
    #[test]
    fn empty_frame_is_a_decline() {
        let bin = fake_daemon(
            "#!/bin/sh\necho \"<<<READY>>>\"\nwhile IFS= read -r l; do echo \"<<<FIX 1>>>\"; echo \"<<<END 1>>>\"; done\n",
        );
        let mut d = DaemonL4::spawn(bin.to_str().unwrap(), "g", "c.bin", 96).unwrap();
        assert!(d.generate_fix("x", "code").is_none());
    }

    /// Pins the fragment-parsing reality with the two REAL daemon responses
    /// captured on 2026-08-17 (riir-train `80601f19` smoke test). The point is
    /// not that the pruner is wrong — it is that edit-only output is sometimes
    /// unparseable, so a G1-passing adapter will still lose a fraction of its
    /// answers to invariant 3. Recorded here so that fraction is understood as
    /// a contract property rather than rediscovered as a bug.
    #[test]
    fn real_daemon_outputs_show_the_edit_only_parse_gap() {
        let pruner = crate::pruner::RustAnalyzerPruner::new();

        // Request 2: balanced, parses.
        let good = "fn g() -> i32 {\n    7\n}";
        assert!(pruner.validate(good, "needless_return").parses, "balanced fix should parse");

        // Request 1: the adapter duplicated a line and dropped the closing
        // brace. Unbalanced — neither parse_file nor the fn _wrap fallback can
        // rescue it, so the seam discards it.
        let bad = "fn f(x: &i32) -> i32 {\n    show(x)\n    show(x)";
        assert!(
            !pruner.validate(bad, "needless_borrow").parses,
            "unbalanced edit-only fragment must NOT parse — if this starts \
             passing, the seam's parse gate changed and invariant 3 needs review"
        );
    }
}
