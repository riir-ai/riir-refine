use super::*;
use std::collections::BTreeMap as Map;

/// Pattern → rev → line count.
type FakeCounts = Map<String, Map<String, usize>>;
/// Pattern → newest-first commit hits.
type FakeCommits = Map<String, Vec<CommitHit>>;

struct FakeHistory {
    resolvable: Vec<String>,
    counts: FakeCounts,
    commits: FakeCommits,
    /// The pickaxe-call witness — the P26 marked-orphaned skip pins this at 0.
    pickaxe_calls: std::cell::Cell<usize>,
}

impl FakeHistory {
    fn new(depths: &[usize]) -> Self {
        Self {
            resolvable: std::iter::once("HEAD".to_string())
                .chain(depths.iter().map(|d| format!("HEAD~{d}")))
                .collect(),
            counts: Map::new(),
            commits: Map::new(),
            pickaxe_calls: std::cell::Cell::new(0),
        }
    }

    fn count(mut self, field: &str, rev: &str, n: usize) -> Self {
        self.counts
            .entry(read_shape_ere(field))
            .or_default()
            .insert(rev.to_string(), n);
        self
    }

    fn commit(mut self, field: &str, subject: &str) -> Self {
        self.commits
            .entry(read_shape_ere(field))
            .or_default()
            .push(CommitHit {
                hash: format!("{:07x}", subject.len()),
                date: "2026-09-01".to_string(),
                subject: subject.to_string(),
            });
        self
    }
}

impl HistoryProbe for FakeHistory {
    fn rev_at_depth(&self, depth: usize) -> Option<String> {
        let rev = if depth == 0 {
            "HEAD".to_string()
        } else {
            format!("HEAD~{depth}")
        };
        self.resolvable.contains(&rev).then_some(rev)
    }

    fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize> {
        self.counts.get(pattern)?.get(rev).copied()
    }

    fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit> {
        self.pickaxe_calls.set(self.pickaxe_calls.get() + 1);
        self.commits
            .get(pattern)
            .map(|c| c.iter().take(max).cloned().collect())
            .unwrap_or_default()
    }
}

const DEPTHS: [usize; 2] = [10, 50];

fn scan_of(src: &str) -> ZeroReadScan {
    crate::config_audit::zero_external_read_fields(&[("lib.rs".to_string(), src.to_string())])
}

const ORPHANED_SRC: &str = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
";

#[test]
fn orphaned_fires_with_optimization_tag() {
    let scan = scan_of(ORPHANED_SRC);
    assert_eq!(scan.fields.len(), 1, "alarm_radius is zero-external-read");
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 0)
        .count("alarm_radius", "HEAD~10", 0)
        .count("alarm_radius", "HEAD~50", 3)
        .commit("alarm_radius", "perf: const-table species behavior values");
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(report.orphaned.len(), 1, "the vanished reader fires");
    assert!(report.stillborn.is_empty());
    let row = &report.orphaned[0];
    assert!(row.likely_by_optimization, "perf subject tags the row");
    let rendered = report.render();
    assert!(rendered.contains("[C1]"), "{rendered}");
    assert!(rendered.contains("LIKELY-BY-OPTIMIZATION"), "{rendered}");
    assert!(rendered.contains("alarm_radius"), "{rendered}");
}

#[test]
fn plain_subject_orphans_without_the_tag() {
    let scan = scan_of(ORPHANED_SRC);
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 0)
        .count("alarm_radius", "HEAD~50", 1)
        .commit("alarm_radius", "docs: update config reference");
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(report.orphaned.len(), 1);
    assert!(!report.orphaned[0].likely_by_optimization);
}

#[test]
fn never_read_is_stillborn_not_orphaned() {
    let scan = scan_of(ORPHANED_SRC);
    let fake = FakeHistory::new(&DEPTHS); // no counts anywhere
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert!(report.orphaned.is_empty(), "no delta, no orphan");
    assert_eq!(report.stillborn.len(), 1, "never read → stillborn");
    assert!(report.render().contains("[stillborn]"));
}

/// P26: a stillborn row whose FIELD doc carries a deliberate-write-only
/// marker downgrades to the weak `[documented-write-only]` annotation —
/// visible, never suppressed (the `[wire-const]` precedent).
#[test]
fn documented_write_only_marker_downgrades_stillborn() {
    let src = r"
pub struct V {
    /// documented-write-only: adjudicated (Issues 142 + 147) forward surface.
    pub f: bool,
}
impl Default for V { fn default() -> Self { Self { f: true } } }
";
    let scan = scan_of(src);
    assert_eq!(scan.marked_write_only.len(), 1, "the field doc is marked");
    let fake = FakeHistory::new(&DEPTHS); // no counts anywhere → stillborn branch
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert!(
        report.stillborn.is_empty(),
        "the marked row downgrades out of stillborn"
    );
    assert_eq!(report.documented_write_only.len(), 1);
    let rendered = report.render();
    assert!(rendered.contains("[documented-write-only]"), "{rendered}");
    assert!(rendered.contains("1 documented-write-only"), "{rendered}");
}

/// P26: a marked ORPHANED row downgrades AND skips the pickaxe leg —
/// the annotation carries no commit evidence, so the expensive git
/// log -S probe is never spent on it.
#[test]
fn documented_write_only_marker_downgrades_orphaned_and_skips_pickaxe() {
    let src = r"
pub struct BehaviorConfig {
    /// forward surface: the YAML designer owns this at authoring time.
    pub alarm_radius: f32,
}
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
";
    let scan = scan_of(src);
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 0)
        .count("alarm_radius", "HEAD~10", 0)
        .count("alarm_radius", "HEAD~50", 3)
        .commit("alarm_radius", "perf: const-table species behavior values");
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(
        fake.pickaxe_calls.get(),
        0,
        "the marked row never reaches the pickaxe"
    );
    assert!(
        report.orphaned.is_empty(),
        "the marked row downgrades out of [C1]"
    );
    assert_eq!(report.documented_write_only.len(), 1);
    assert!(report.render().contains("[documented-write-only]"));
}

/// The marker counts only on the FIELD doc — a struct-level doc speaks
/// for the whole type, not for one member (the config-audit
/// `struct_doc_marker_does_not_downgrade_field` parity).
#[test]
fn struct_doc_marker_does_not_downgrade_orphan_row() {
    let src = r"
/// deliberate: write-only config surface (struct-level doc).
pub struct W { pub g: u32 }
impl Default for W { fn default() -> Self { Self { g: 1 } } }
";
    let scan = scan_of(src);
    assert!(
        scan.marked_write_only.is_empty(),
        "struct doc does not mark one field"
    );
    let fake = FakeHistory::new(&DEPTHS)
        .count("g", "HEAD", 0)
        .count("g", "HEAD~50", 2);
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(report.orphaned.len(), 1, "an unmarked row stays [C1]");
    assert!(report.documented_write_only.is_empty());
}

/// The riir-chain `GuardVerdict` class: a marked field whose ONLY reads
/// are its own test pins — grep counts them at HEAD, so the row lands
/// in the silent arm (never stillborn, never orphaned). The marker
/// check must reach the silent arm too, or owner-marked forward
/// surfaces are invisible to the report entirely.
#[test]
fn documented_write_only_marker_reaches_the_silent_arm() {
    let src = r"
pub struct V {
    /// documented-write-only: adjudicated (Issues 142 + 147) forward surface.
    pub f: bool,
}
impl Default for V { fn default() -> Self { Self { f: true } } }
";
    let scan = scan_of(src);
    assert_eq!(scan.marked_write_only.len(), 1, "the field doc is marked");
    // reads EXIST at HEAD and every depth (the test pins) — no delta.
    let fake = FakeHistory::new(&DEPTHS)
        .count("f", "HEAD", 2)
        .count("f", "HEAD~10", 2)
        .count("f", "HEAD~50", 2);
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert!(report.stillborn.is_empty(), "reads at HEAD → not stillborn");
    assert!(report.orphaned.is_empty(), "no delta → not orphaned");
    assert_eq!(
        report.documented_write_only.len(),
        1,
        "the marked silent row downgrades"
    );
    let rendered = report.render();
    assert!(rendered.contains("[documented-write-only]"), "{rendered}");
}

/// The silent arm stays silent for an UNMARKED field — the extension
/// converts only owner-marked rows, never manufactures findings.
#[test]
fn unmarked_silent_row_stays_counted_only() {
    let scan = scan_of(ORPHANED_SRC);
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 1)
        .count("alarm_radius", "HEAD~10", 1)
        .count("alarm_radius", "HEAD~50", 1);
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert!(report.documented_write_only.is_empty());
    assert!(report.stillborn.is_empty());
    assert!(report.orphaned.is_empty());
    let rendered = report.render();
    assert!(
        rendered.contains(" 1 silent (no vanished-reader evidence)"),
        "{rendered}"
    );
}

/// P26 scope: wire-const is a different class and STAYS — a marked field
/// that is also constant-at-every-site renders [wire-const], not the
/// annotation (consumed verdicts outrank the marker).
#[test]
fn wire_const_beats_the_marker() {
    let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EpochSummary {
    pub advisory: u32,
    /// deliberate: pinned protocol constant, echoed verbatim.
    pub provenance_verified: bool,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.count,
        provenance_verified: true,
    };
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
"#;
    let scan = scan_of(src);
    assert!(scan.marked_write_only.contains(&(
        "EpochSummary".to_string(),
        "provenance_verified".to_string()
    )));
    let wire = wire_of(src);
    let fake = FakeHistory::new(&DEPTHS); // zero read-shaped occurrences everywhere
    let report = classify_with_probe(&scan, &fake, &wire, &DEPTHS);
    assert_eq!(report.wire_const.len(), 1, "wire-const keeps the row");
    assert!(
        report.documented_write_only.is_empty(),
        "consumed outranks marked"
    );
}

#[test]
fn own_impl_reads_with_no_delta_stay_silent() {
    // validate() reads self.alarm_radius NOW; the same read existed at
    // every probe — delta 0, not stillborn (head count > 0) → silent.
    let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
impl BehaviorConfig {
    pub fn validate(&self) -> bool { self.alarm_radius > 0.0 }
}
";
    let scan = scan_of(src);
    assert_eq!(scan.fields.len(), 1);
    assert_eq!(scan.fields[0].self_impl_reads, 1);
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 1)
        .count("alarm_radius", "HEAD~10", 1)
        .count("alarm_radius", "HEAD~50", 1);
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert!(
        report.orphaned.is_empty(),
        "no delta — validate then and now"
    );
    assert!(report.stillborn.is_empty(), "head count > 0");
    let rendered = report.render();
    assert!(rendered.contains("1 silent"), "{rendered}");
}

#[test]
fn external_reader_removes_the_field_from_the_candidate_set() {
    // The FN direction of name-global matching: any read of the name
    // (here a real external one) keeps external > 0 — no candidate.
    let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
pub fn alarm_of(c: &BehaviorConfig) -> f32 { c.alarm_radius }
";
    let scan = scan_of(src);
    assert!(scan.fields.is_empty(), "external read keeps it alive");
}

#[test]
fn reader_plus_validate_delta_fires() {
    // The canonical shape: consumer read existed then, only validate
    // remains now → HEAD 1, probe 2 → delta fires.
    let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
impl BehaviorConfig {
    pub fn validate(&self) -> bool { self.alarm_radius > 0.0 }
}
";
    let scan = scan_of(src);
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 1)
        .count("alarm_radius", "HEAD~10", 2)
        .count("alarm_radius", "HEAD~50", 2)
        .commit("alarm_radius", "perf: inline the species table");
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(
        report.orphaned.len(),
        1,
        "consumer vanished, validate stayed"
    );
    assert!(report.orphaned[0].likely_by_optimization);
}

#[test]
fn same_name_on_another_type_masks_the_candidate() {
    // The documented FN direction: another struct's same-named field is
    // read — the syn tally is name-global, so BOTH look read.
    let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
pub struct OtherConfig { pub alarm_radius: f32 }
pub fn other(o: &OtherConfig) -> f32 { o.alarm_radius }
";
    let scan = scan_of(src);
    assert!(
        scan.fields
            .iter()
            .all(|f| f.candidate.field_name != "alarm_radius"),
        "name-global read masks the orphan candidate"
    );
}

#[test]
fn read_shape_ere_binds_the_tail() {
    let p = read_shape_ere("alarm_radius");
    assert_eq!(p, r"\.alarm_radius([^[:alnum:]_]|$)");
    // `.alarm_radius_longer` must NOT match the pattern as a whole word…
    // (the ERE would need a backreference to assert that; the tail class
    // is the portable approximation — pinned by the e2e test below.)
}

#[test]
fn subject_vocabulary_is_case_insensitive() {
    assert!(subject_looks_like_optimization("PERF: const table"));
    assert!(subject_looks_like_optimization("Refactor the boot path"));
    assert!(!subject_looks_like_optimization("docs: readme"));
}

/// The canonical C1 shape (Plan 126's own control): a serde-backed
/// designer config with NO `impl Default`, constructed by handwritten
/// literals, its field never member-read — the field MUST enter the
/// candidate set via the serde-literal extension.
const SERDE_LITERAL_SRC: &str = r#"
#[derive(serde::Deserialize)]
pub struct BehaviorConfig { #[serde(default = "default_alarm_radius")] pub alarm_radius: f32 }
fn default_alarm_radius() -> f32 { 15.0 }
pub fn hardcoded_registry() -> BehaviorConfig {
    BehaviorConfig { alarm_radius: 30.0 }
}
"#;

#[test]
fn serde_literal_constructor_is_candidate() {
    let scan = scan_of(SERDE_LITERAL_SRC);
    assert_eq!(
        scan.fields.len(),
        1,
        "the serde-literal field enters the set"
    );
    assert_eq!(scan.fields[0].candidate.field_name, "alarm_radius");
    assert_eq!(scan.fields[0].candidate.struct_name, "BehaviorConfig");
    // And with no history evidence it reads stillborn (the canonical
    // case's honest verdict — the pre-fix tree has zero read-shaped
    // occurrences at every commit).
    let fake = FakeHistory::new(&DEPTHS);
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(report.stillborn.len(), 1);
    assert!(report.orphaned.is_empty());
}

#[test]
fn serde_literal_orphaned_when_a_reader_vanished() {
    let scan = scan_of(SERDE_LITERAL_SRC);
    let fake = FakeHistory::new(&DEPTHS)
        .count("alarm_radius", "HEAD", 0)
        .count("alarm_radius", "HEAD~50", 2)
        .commit("alarm_radius", "perf: inline species table");
    let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
    assert_eq!(
        report.orphaned.len(),
        1,
        "the vanished reader fires on the serde-literal candidate"
    );
    assert!(report.orphaned[0].likely_by_optimization);
}

#[test]
fn serialize_only_struct_is_not_a_candidate() {
    // Serialize-only: the derive reads the fields — every one would
    // read as stillborn. Deliberately outside the extension.
    let src = r"
#[derive(serde::Serialize)]
pub struct WireOut { pub seq: u32 }
pub fn emit() -> WireOut { WireOut { seq: 1 } }
";
    let scan = scan_of(src);
    assert!(
        scan.fields.is_empty(),
        "Serialize-only structs are excluded"
    );
}

/// The Issue 934 regression pin (the `ConversationEntry` false
/// positive): a Ser+De struct whose field carries
/// `#[serde(skip_serializing_if)]` is conditionally-emitted OUTPUT
/// SCHEMA — the Serialize derive reads it when the condition holds,
/// invisibly to the member-read tally. Never a C1 candidate.
#[test]
fn skip_serializing_if_field_is_not_a_candidate() {
    let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConversationEntry {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_tick: Option<u64>,
}
pub fn make() -> ConversationEntry {
    ConversationEntry { state: "Failed".into(), deadline_tick: None }
}
pub fn state_of(e: &ConversationEntry) -> &str { &e.state }
"#;
    let scan = scan_of(src);
    assert!(
        scan.fields.is_empty(),
        "skip_serializing_if fields are excluded (derive-read schema)"
    );
}

/// The FN guard on the same boundary (the `BehaviorConfig` canonical
/// family): a plain `#[serde(default)]` field in a struct deriving BOTH
/// Serialize AND Deserialize STAYS a candidate — persistence-mirror
/// surface whose functional read is exactly what the report measures.
/// The Issue 934 sample: every skip-attrs field was an FP, every
/// plain-default field in the same derive shape a TP — the boundary is
/// the attr, NOT the derive pair.
#[test]
fn plain_serde_default_in_ser_de_struct_stays_candidate() {
    let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BehaviorConfig {
    #[serde(default = "default_alarm_radius")]
    pub alarm_radius: f32,
}
pub fn make() -> BehaviorConfig { BehaviorConfig { alarm_radius: 15.0 } }
"#;
    let scan = scan_of(src);
    assert_eq!(
        scan.fields.len(),
        1,
        "plain serde(default) fields stay candidates"
    );
    assert_eq!(scan.fields[0].candidate.field_name, "alarm_radius");
}

// ── Wire-consumption fixtures (the 2026-09-12 measured WIRE-FP repair;
// shapes pin the cross-repo probe evidence — the intake row) ──

/// Drive the wire pass directly over one source.
fn wire_of(src: &str) -> WireConsumption {
    wire_consumption(&[("lib.rs".to_string(), src.to_string())])
}

/// The delegation.rs shape: `let tx = DelegateTx { … };` then
/// `serde_json::to_vec(&tx)` in the SAME fn — the field value is a
/// variable/call (`delegator_pubkey` from keypair) → CLEARED.
#[test]
fn wire_literal_in_serde_fn_is_cleared() {
    let src = r"
#[derive(serde::Serialize)]
pub struct DelegateTx {
    pub delegator_pubkey: String,
    pub amount: u64,
}
pub fn build(keypair: &Keypair, amount: u64) -> Vec<u8> {
    let tx = DelegateTx {
        delegator_pubkey: keypair.public(),
        amount,
    };
    serde_json::to_vec(&tx).unwrap()
}
";
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("DelegateTx".to_string(), "delegator_pubkey".to_string())),
        "literal + to_vec in one fn clears the variable-valued field"
    );
    assert!(wire.wire_const.is_empty(), "no constant-valued field here");
}

/// The dao-soak `main()` shape: the literal and the serialize call FAR
/// APART inside one big fn — fn scope, not adjacency.
#[test]
fn wire_serde_call_far_apart_in_same_fn_clears() {
    let src = r#"
#[derive(serde::Serialize)]
pub struct EpochSummary {
    pub advisory: u32,
    pub novel: u32,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.advisory_count,
        novel: verdict.novel_count,
    };
    // … many lines …
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
fn decide() -> Verdict { Verdict::default() }
"#;
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("EpochSummary".to_string(), "advisory".to_string()))
    );
}

/// The riir-kat queue-writer shape (the 2026-09-16 fn-A/fn-B intake,
/// 16-row concentration): the literal is built in the FLOW fn whose own
/// body has NO serde to_* call (`json!` + Value `.to_string()` are not
/// serde paths), and the SERIALIZING fn receives the struct as a
/// concrete `&VoteIntent` param. Fn-scoped co-occurrence cannot join
/// them; the per-param link does.
#[test]
fn wire_param_serialized_in_sibling_fn_clears() {
    let src = r#"
#[derive(serde::Serialize)]
pub struct VoteIntent {
    pub vote_version: u32,
    pub queued_unix: u64,
}
pub fn vote_flow(now_unix: u64) -> std::io::Result<()> {
    let intent = VoteIntent { vote_version: VOTE_VERSION, queued_unix: now_unix };
    queue_vote_intent(std::path::Path::new("/tmp"), &intent)
}
pub fn queue_vote_intent(
    root: &std::path::Path,
    intent: &VoteIntent,
) -> std::io::Result<()> {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(root)?;
    writeln!(f, "{}", serde_json::to_string(intent).unwrap_or_default())
}
pub const VOTE_VERSION: u32 = 1;
"#;
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("VoteIntent".to_string(), "vote_version".to_string())),
        "a const-PATH field value is not a literal — the named const could change"
    );
    assert!(
        wire.cleared
            .contains(&("VoteIntent".to_string(), "queued_unix".to_string())),
        "variable-valued field clears through the sibling-fn param link"
    );
}

/// The riir-reflex `FusedGateRecommendation` shape (2026-09-23 intake):
/// the struct literal is built in the producer fn, and the SERIALIZING
/// fn is a test that binds the value from the producer's RETURN (`let
/// fused = recommend_fused_gate(..); serde_json::to_string(&fused)`).
/// Neither fn-scoped co-occurrence (the literal lives in the producer,
/// which has no serde call) nor the concrete-param link (the test takes
/// no param) can join them — the return-value link does.
#[test]
fn wire_return_value_serialized_in_test_fn_clears() {
    let src = r#"
#[derive(serde::Serialize)]
pub enum Posture {
    Percentile { rho: f32 },
}
#[derive(serde::Serialize)]
pub struct Rec {
    pub threshold: f64,
}
#[derive(serde::Serialize)]
pub struct FusedGateRecommendation {
    pub posture: Posture,
    pub score: Option<Rec>,
    pub distance: Option<Rec>,
}
pub fn recommend_fused_gate(rho: f32) -> FusedGateRecommendation {
    let posture = Posture::Percentile { rho };
    FusedGateRecommendation { posture, score: None, distance: None }
}
#[test]
fn serde_metadata_shape() {
    let fused = recommend_fused_gate(0.3);
    let json = serde_json::to_string(&fused).unwrap();
    assert!(json.contains("posture"));
}
"#;
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("FusedGateRecommendation".to_string(), "posture".to_string())),
        "a variable-valued field clears through the return-value link"
    );
    assert!(
        wire.cleared
            .contains(&("FusedGateRecommendation".to_string(), "score".to_string())),
        "Option-valued fields clear with the struct"
    );
    assert!(
        wire.cleared.contains(&(
            "FusedGateRecommendation".to_string(),
            "distance".to_string()
        )),
        "the second Option field clears with the struct"
    );
}

/// The return-value link's guards, each a refusal (the conservative
/// direction): (1) the serialized local is NOT the call-bound one — a
/// call-bound local merely EXISTING in the fn clears nothing; (2) the
/// callee is not a bare same-file free fn (`Rec::make` is associated,
/// unknown to the resolution map) → no clearance; (3) a shadowed binding
/// refuses the name entirely, even though one of the two inits is a
/// known producer call.
#[test]
fn wire_return_value_link_stays_type_exact() {
    let src = r"
#[derive(serde::Serialize)]
pub struct Dead1 {
    pub a: u8,
}
#[derive(serde::Serialize)]
pub struct Dead2 {
    pub b: u8,
}
#[derive(serde::Serialize)]
pub struct Dead3 {
    pub c: u8,
}
pub fn make1() -> Dead1 {
    Dead1 { a: 1 }
}
impl Rec2 {
    pub fn make() -> Dead2 {
        Dead2 { b: 2 }
    }
}
pub struct Rec2;
pub fn make3() -> Dead3 {
    Dead3 { c: 3 }
}
pub struct Other;
#[test]
fn wrong_local_serialized() {
    let made = make1();
    let other = 5u8;
    let _ = serde_json::to_string(&other);
    let _ = made;
}
#[test]
fn associated_callee_unknown() {
    let made = Rec2::make();
    let _ = serde_json::to_string(&made);
}
#[test]
fn shadowed_binding_refuses() {
    let made = make3();
    let made = Other;
    let _ = serde_json::to_string(&made);
}
";
    let wire = wire_of(src);
    assert!(
        !wire
            .cleared
            .contains(&("Dead1".to_string(), "a".to_string())),
        "a call-bound local that is not the serialized one clears nothing"
    );
    assert!(
        !wire
            .cleared
            .contains(&("Dead2".to_string(), "b".to_string())),
        "an associated-fn callee is unknown to the same-file free-fn map"
    );
    assert!(
        !wire
            .cleared
            .contains(&("Dead3".to_string(), "c".to_string())),
        "a shadowed binding refuses the name entirely"
    );
}

/// The `serde_json::json!`-as-serializer shape (the 2026-09-17 intake,
/// riir-kat `shop_seed_body`): the macro IS the serializer — each
/// interpolated non-literal arg goes through `to_value` at runtime, no
/// `to_*` path call exists anywhere. Before this repair the interpolating
/// fn was not a serde fn at all and the param's fields read `[stillborn]`.
/// The literal lives in the CALLER (the flow fn) — the sibling-fn shape
/// with json! as the serializer half.
#[test]
fn json_macro_interpolated_param_clears() {
    let src = r#"
#[derive(serde::Serialize)]
pub struct ShopSeedItemWire {
    pub item_id: u64,
    pub price_micro: i64,
}
pub fn seed_flow(item_id: u64, price_micro: i64) -> String {
    let item = ShopSeedItemWire { item_id, price_micro };
    shop_seed_body(7, &[item])
}
pub fn shop_seed_body(seq: u64, items: &[ShopSeedItemWire]) -> String {
    serde_json::json!({"seq": seq, "items": items}).to_string()
}
"#;
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("ShopSeedItemWire".to_string(), "price_micro".to_string())),
        "json!-interpolated param fields are wire-consumed (the recorded FP specimen)"
    );
    assert!(
        wire.cleared
            .contains(&("ShopSeedItemWire".to_string(), "item_id".to_string())),
        "both fields of the interpolated slice param clear"
    );
}

/// The same macro, with the param NOT interpolated — the conservative
/// direction must survive: a fn that builds a json! body not mentioning
/// the param does not clear a merely-received param's type. (A bare
/// `items.len()` INSIDE the json! tokens WOULD mention `items` — the
/// same name-appearance stance the `to_*` arm documents — so this
/// fixture keeps the ident out of the tokens entirely.)
#[test]
fn json_macro_without_param_mention_does_not_clear() {
    let src = r#"
#[derive(serde::Serialize)]
pub struct ShopSeedItemWire {
    pub item_id: u64,
    pub price_micro: i64,
}
pub fn shop_seed_body(seq: u64, items: &[ShopSeedItemWire], count: usize) -> String {
    let _ = items; // received but NOT interpolated
    serde_json::json!({"seq": seq, "count": count}).to_string()
}
"#;
    let wire = wire_of(src);
    assert!(
        !wire
            .cleared
            .contains(&("ShopSeedItemWire".to_string(), "price_micro".to_string())),
        "a json! body that does not mention the param must not clear it"
    );
}

/// Only the crate-rooted `serde_json::json!` counts — a bare `json!`
/// (one import away from a local macro of the same name) does not.
#[test]
fn bare_json_macro_does_not_clear() {
    let src = r#"
#[derive(serde::Serialize)]
pub struct ShopSeedItemWire {
    pub item_id: u64,
    pub price_micro: i64,
}
pub fn shop_seed_body(seq: u64, items: &[ShopSeedItemWire]) -> String {
    json!({"seq": seq, "items": items}).to_string()
}
"#;
    let wire = wire_of(src);
    assert!(
        !wire
            .cleared
            .contains(&("ShopSeedItemWire".to_string(), "price_micro".to_string())),
        "the conservative crate-rooted-only rule: bare json! is not a serializer signal"
    );
}

/// The per-param mention check is load-bearing: a fn that serializes an
/// UNRELATED value while merely RECEIVING a param of the type must not
/// clear it (the conservative direction survives inside the new arm).
/// Both structs carry literals — the difference is solely which param
/// the serializer's argument mentions.
#[test]
fn wire_param_only_cleared_when_actually_serialized() {
    let src = r"
#[derive(serde::Serialize)]
pub struct StakeIntent {
    pub stake_version: u32,
}
#[derive(serde::Serialize)]
pub struct Misc {
    pub note: String,
}
pub fn make_stake(v: u32) -> StakeIntent {
    StakeIntent { stake_version: v }
}
pub fn make_misc(note: String) -> Misc {
    Misc { note }
}
pub fn other(x: &StakeIntent, misc: &Misc) -> String {
    let s = serde_json::to_string(misc).unwrap();
    let _ = x;
    s
}
";
    let wire = wire_of(src);
    assert!(
        !wire
            .cleared
            .contains(&("StakeIntent".to_string(), "stake_version".to_string())),
        "a serde call that does not serialize the param does not clear it"
    );
    assert!(
        wire.cleared
            .contains(&("Misc".to_string(), "note".to_string())),
        "the actually-serialized param's type clears through its literal sites"
    );
}

/// The `payload_json` shape (the wrapping clause): the INNER type has NO
/// direct literal in any serde fn (its literals live only in builders /
/// tests) — it is consumed transitively via a `Vec<T>` field of a
/// wrapper that IS literal-constructed in a serde fn.
#[test]
fn wire_vec_wrapped_type_cleared_transitively() {
    let src = r"
#[derive(serde::Serialize)]
pub struct ReportTarget {
    pub manifest_hash: String,
}
#[derive(serde::Serialize)]
pub struct SelfReportWire {
    pub targets: Vec<ReportTarget>,
}
pub fn payload_json(targets: &[ReportTarget]) -> String {
    let wire = SelfReportWire { targets: targets.to_vec() };
    serde_json::to_string(&wire).unwrap()
}
pub fn probe_target() -> ReportTarget {
    ReportTarget { manifest_hash: compute_manifest_hash() }
}
fn compute_manifest_hash() -> String { String::new() }
";
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("ReportTarget".to_string(), "manifest_hash".to_string())),
        "the Vec-wrapped inner type's call-valued field clears transitively"
    );
}

/// The `provenance_verified` shape (the 1/8 measured swallow, kept
/// VISIBLE): wire-consumed but HARDCODED at its only construction site —
/// `[wire-const]`, never cleared.
#[test]
fn wire_const_constant_at_every_site_is_flagged_weak() {
    let src = r#"
#[derive(serde::Serialize)]
pub struct EpochSummary {
    pub advisory: u32,
    pub provenance_verified: bool,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.count,
        provenance_verified: true,
    };
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
"#;
    let wire = wire_of(src);
    assert!(
        wire.wire_const.iter().any(|k| k
            == &(
                "EpochSummary".to_string(),
                "provenance_verified".to_string()
            )),
        "constant-at-every-site → wire-const, not cleared"
    );
    assert!(!wire.cleared.contains(&(
        "EpochSummary".to_string(),
        "provenance_verified".to_string()
    )));
    assert!(
        wire.cleared
            .contains(&("EpochSummary".to_string(), "advisory".to_string())),
        "the variable-valued sibling field still clears"
    );
}

/// The riir-kat FP class (2026-09-21 intake): ALL in-repo construction
/// sites sit inside `#[cfg(test)]` fixtures with literal constants while
/// the production constructors live OUTSIDE the scanned set (another
/// repo's client builds `ClaimRequest { queued_unix: now() }`). Test-only
/// sites are no evidence for `[wire-const]` — the (type, field) renders
/// NOTHING (not cleared either: nothing was measured).
#[test]
fn wire_const_test_only_sites_render_nothing() {
    let src = r"
#[derive(serde::Serialize)]
pub struct ClaimRequest {
    pub queued_unix: u64,
}
pub fn enqueue(req: &ClaimRequest) -> Vec<u8> {
    serde_json::to_vec(req).unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn claim_queues_locally() {
        let req = ClaimRequest { queued_unix: 1726900000 };
        let bytes = enqueue(&req);
        assert!(!bytes.is_empty());
    }
}
";
    let wire = wire_of(src);
    assert!(
        !wire
            .wire_const
            .contains(&("ClaimRequest".to_string(), "queued_unix".to_string())),
        "test-only const sites must not arm wire-const"
    );
    assert!(
        !wire
            .cleared
            .contains(&("ClaimRequest".to_string(), "queued_unix".to_string())),
        "test-only sites are no evidence in EITHER direction"
    );
}

/// Production const + test VAR: the test site is no evidence, so the
/// production const rules — `[wire-const]` fires (previously the test
/// var site would have wrongly CLEARED it).
#[test]
fn wire_test_var_prod_const_is_wire_const() {
    let src = r"
#[derive(serde::Serialize)]
pub struct Pulse {
    pub era: u32,
}
pub fn send(p: &Pulse) -> Vec<u8> {
    serde_json::to_vec(p).unwrap()
}
pub fn prod_pulse() -> Pulse {
    Pulse { era: 7 }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t() {
        let p = Pulse { era: current_era_for_test() };
        let _ = send(&p);
    }
    fn current_era_for_test() -> u32 { 0 }
}
";
    let wire = wire_of(src);
    assert!(
        wire.wire_const
            .contains(&("Pulse".to_string(), "era".to_string())),
        "the production const site rules; the test var site is no evidence"
    );
}

/// Test const + production VAR: the production var clears — test sites
/// still cannot arm the class against real production evidence.
#[test]
fn wire_test_const_prod_var_clears() {
    let src = r"
#[derive(serde::Serialize)]
pub struct Telemetry {
    pub session: u64,
}
pub fn send(t: &Telemetry) -> Vec<u8> {
    serde_json::to_vec(t).unwrap()
}
pub fn prod(now: u64) -> Telemetry {
    Telemetry { session: now }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t() {
        let x = Telemetry { session: 1 };
        let _ = send(&x);
    }
}
";
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("Telemetry".to_string(), "session".to_string())),
        "production variable site clears; the test const site cannot arm wire-const"
    );
    assert!(
        !wire
            .wire_const
            .contains(&("Telemetry".to_string(), "session".to_string()))
    );
}

/// A struct literal in a NON-serde fn is NOT wire evidence — the field
/// stays stillborn (no serializer touches the type anywhere: not in the
/// literal's fn, not through any param-serializing sibling fn — the
/// residual conservative gap).
#[test]
fn wire_literal_in_plain_fn_stays_flagged() {
    let src = r"
#[derive(serde::Serialize)]
pub struct PlainStruct {
    pub knob: f32,
}
pub fn build() -> PlainStruct {
    PlainStruct { knob: 1.5 }
}
";
    let wire = wire_of(src);
    assert!(wire.cleared.is_empty(), "no serde call → no wire evidence");
    assert_eq!(wire.wire_const, []);
}

/// Constant at one site, variable at another → NOT wire-const (the
/// receiver can be seeing the variable value; the 7/8 measured class).
#[test]
fn wire_mixed_const_and_variable_sites_clear() {
    let src = r"
#[derive(serde::Serialize)]
pub struct Tx {
    pub gas_budget: u64,
}
pub fn emit(gas: u64) -> Vec<u8> {
    let tx = Tx { gas_budget: gas };
    serde_json::to_vec(&tx).unwrap()
}
pub fn fallback() -> Tx {
    Tx { gas_budget: 5 }
}
";
    let wire = wire_of(src);
    assert!(
        wire.cleared
            .contains(&("Tx".to_string(), "gas_budget".to_string())),
        "any non-constant site defeats wire-const"
    );
    assert_eq!(wire.wire_const, []);
}

// ── Issue 109: foreign literals + generic serde bridges ──

/// Specimen A, wire side: a FOREIGN literal inside a serde fn must not
/// mark the scanned set's same-named struct wire-consumed (the false
/// CLEAR direction — worse than a noisy row).
#[test]
fn foreign_literal_in_serde_fn_does_not_mark_wire_type() {
    let local = r"
#[derive(serde::Serialize)]
pub struct Node { pub id: u32 }
";
    let other = r"
use bevy::prelude::*;
pub fn emit() -> String {
    let n = Node { flex_direction: FlexDirection::Column };
    serde_json::to_string(&n).unwrap()
}
";
    let wire = wire_consumption(&[
        ("crates/graph/src/types.rs".to_string(), local.to_string()),
        ("crates/showcase/src/ui.rs".to_string(), other.to_string()),
    ]);
    assert!(
        !wire
            .cleared
            .contains(&("Node".to_string(), "flex_direction".to_string())),
        "the foreign literal is not wire evidence for the local Node"
    );
    assert!(
        !wire
            .cleared
            .contains(&("Node".to_string(), "id".to_string())),
        "the local Node itself was never literal-built in a serde fn"
    );
}

/// Specimen B: the seal-game-editor shape — a generic serde bridge
/// (`row_to_mapping<T: serde::Serialize>(row: &T)` serializing the VALUE
/// param typed over the bounded param) whose call sites name the
/// concrete types by turbofish (`save_t::<CraftRow>(m)` on the call,
/// `row_to_mapping(&Repo::<CraftRow>::get(..))` inside the args).
#[test]
fn generic_serde_bridge_marks_turbofish_types() {
    let src = r"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CraftRow { pub updated_at: i64 }
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoneStepRow { pub updated_at: i64 }

fn row_to_mapping<T: serde::Serialize>(row: &T) -> Option<serde_yaml::Mapping> {
    match serde_yaml::to_value(row).ok()? {
        serde_yaml::Value::Mapping(m) => Some(m),
        _ => None,
    }
}

fn emit_t<T>(row: &T) -> String
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    serde_json::to_string(row).unwrap()
}

pub fn cluster_load(id: i64) -> Option<serde_yaml::Mapping> {
    let store = open_store();
    row_to_mapping(&Repo::<CraftRow>::get(&store, id).ok()??)
}
pub fn emit_stone(row: &StoneStepRow) -> String {
    emit_t::<StoneStepRow>(row)
}
fn open_store() -> Store { Store }
struct Store;
struct Repo<T> { _t: std::marker::PhantomData<T> }
impl<T> Repo<T> {
    fn get(_s: &Store, _id: i64) -> Result<T, E> { Err(E) }
}
struct E;
";
    let wire = wire_of(src);
    assert!(
        wire.bridge_types.contains("CraftRow"),
        "the arg-turbofish type is bridge-consumed ({:?})",
        wire.bridge_types
    );
    assert!(
        wire.bridge_types.contains("StoneStepRow"),
        "the call-turbofish type is bridge-consumed (where-clause bound)"
    );
}

/// A fn that carries a Serialize param but serializes an UNRELATED
/// concrete value is NOT a bridge (the param-mention guard).
#[test]
fn serialize_param_with_unrelated_concrete_target_is_not_a_bridge() {
    let src = r"
#[derive(serde::Serialize)]
pub struct Other { pub seq: u32 }
pub fn helper<T: serde::Serialize>(_t: &T) -> String {
    let o = Other { seq: 1 };
    serde_json::to_string(&o).unwrap()
}
pub fn call(x: &Other) -> String { helper::<Other>(x) }
";
    let wire = wire_of(src);
    assert!(
        !wire.bridge_types.contains("Other"),
        "an unrelated concrete serialization is not a bridge"
    );
}

/// End-to-end: a stillborn row on a bridge-consumed struct counts as
/// bridge-cleared (counted, not listed — the measured-FP posture of
/// `wire_cleared`).
#[test]
fn bridge_type_stillborn_counts_bridge_cleared() {
    let src = r"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CraftRow { pub updated_at: i64 }
fn row_to_mapping<T: serde::Serialize>(row: &T) -> Option<i64> {
    match serde_yaml::to_value(row).ok()? { _ => None }
}
pub fn load(id: i64) -> Option<i64> {
    let store = open_store();
    row_to_mapping(&Repo::<CraftRow>::get(&store, id).ok()??)
}
pub fn map_row(id: i64) -> CraftRow {
    CraftRow { updated_at: id }
}
fn open_store() -> Store { Store }
struct Store;
struct Repo<T> { _t: std::marker::PhantomData<T> }
impl<T> Repo<T> { fn get(_s: &Store, _id: i64) -> Result<T, E> { Err(E) } }
struct E;
";
    let scan = scan_of(src);
    assert_eq!(
        scan.fields.len(),
        1,
        "updated_at is a zero-external-read candidate"
    );
    let wire = wire_of(src);
    let fake = FakeHistory::new(&DEPTHS);
    let report = classify_with_probe(&scan, &fake, &wire, &DEPTHS);
    assert!(report.stillborn.is_empty(), "bridge-cleared, not stillborn");
    assert_eq!(report.bridge_cleared, 1);
    let rendered = report.render();
    assert!(rendered.contains("1 bridge-cleared"), "{rendered}");
}

/// End-to-end: an ORPHANED row on a bridge-consumed struct keeps its
/// history evidence but renders at wire-dto weak severity (the receiver
/// consumes the serialized field — the in-repo read loss is evidence,
/// not a death verdict).
#[test]
fn bridge_type_orphaned_row_is_wire_dto_weak() {
    let src = r"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CraftRow { pub updated_at: i64 }
fn row_to_mapping<T: serde::Serialize>(row: &T) -> Option<i64> {
    match serde_yaml::to_value(row).ok()? { _ => None }
}
pub fn load(id: i64) -> Option<i64> {
    let store = open_store();
    row_to_mapping(&Repo::<CraftRow>::get(&store, id).ok()??)
}
pub fn map_row(id: i64) -> CraftRow {
    CraftRow { updated_at: id }
}
fn open_store() -> Store { Store }
struct Store;
struct Repo<T> { _t: std::marker::PhantomData<T> }
impl<T> Repo<T> { fn get(_s: &Store, _id: i64) -> Result<T, E> { Err(E) } }
struct E;
";
    let scan = scan_of(src);
    let wire = wire_of(src);
    let fake = FakeHistory::new(&DEPTHS)
        .count("updated_at", "HEAD", 0)
        .count("updated_at", "HEAD~50", 2)
        .commit("updated_at", "refactor: move the form to a generic bridge");
    let report = classify_with_probe(&scan, &fake, &wire, &DEPTHS);
    assert_eq!(report.orphaned.len(), 1, "the history evidence stands");
    assert!(
        report.orphaned[0].bridge_weak,
        "the row renders at wire-dto weak"
    );
    let rendered = report.render();
    assert!(rendered.contains("1 wire-dto weak"), "{rendered}");
    assert!(rendered.contains("wire-dto WEAK"), "{rendered}");
}

/// End-to-end through the classifier: a wire-cleared field drops out of
/// stillborn; a wire-const field renders its own weak row. (Ser+De —
/// the measured family: the candidate net's serde-literal extension
/// covers Ser+De structs; pure-Serialize stays outside it.)
#[test]
fn wire_clears_stillborn_and_reports_wire_const_end_to_end() {
    let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EpochSummary {
    pub advisory: u32,
    pub provenance_verified: bool,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.count,
        provenance_verified: true,
    };
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
"#;
    let scan = scan_of(src);
    assert_eq!(
        scan.fields.len(),
        2,
        "both serde-literal fields are candidates"
    );
    let wire = wire_of(src);
    let fake = FakeHistory::new(&DEPTHS); // zero read-shaped occurrences everywhere
    let report = classify_with_probe(&scan, &fake, &wire, &DEPTHS);
    assert_eq!(report.wire_cleared, 1, "advisory (variable-valued) clears");
    assert!(report.stillborn.is_empty(), "cleared rows leave stillborn");
    assert_eq!(
        report.wire_const.len(),
        1,
        "provenance_verified stays as wire-const"
    );
    let rendered = report.render();
    assert!(rendered.contains("wire-cleared"));
    assert!(rendered.contains("[wire-const]"));
    assert!(rendered.contains("provenance_verified"));
}

#[test]
fn self_literal_in_default_impl_not_duplicated() {
    // A struct with BOTH a Default impl and serde markers: the `Self {}`
    // literal inside the impl is collected once (scan_core resolves the
    // impl type; the serde collector skips `Self`).
    let src = r"
#[derive(serde::Deserialize)]
pub struct Dual { pub knob: f32 }
impl Default for Dual {
    fn default() -> Self { Self { knob: 1.0 } }
}
";
    let scan = scan_of(src);
    assert_eq!(scan.fields.len(), 1, "exactly one candidate row");
}

// ── Issue 109 specimen A: per-file foreign-literal attribution ──

/// The riir-shader shape: a local serde-backed `Node` declared in one
/// crate's file, bevy's `Node { … }` literals in another file that
/// reaches `bevy_ui::Node` through a FOREIGN GLOB — the file-global name
/// match used to count the foreign literal's fields as candidates for
/// the local struct (4 stillborn rows, zero true findings).
#[test]
fn foreign_glob_literal_is_not_a_local_candidate() {
    let graph = r"
#[derive(serde::Deserialize)]
pub struct Node { pub id: u32 }
";
    let ui = r"
use bevy::prelude::*;
pub fn panel() { let _n = Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0) }; }
";
    let scan = crate::config_audit::zero_external_read_fields(&[
        ("crates/graph/src/types.rs".to_string(), graph.to_string()),
        ("crates/showcase/src/ui.rs".to_string(), ui.to_string()),
    ]);
    assert!(
        scan.fields
            .iter()
            .all(|f| f.candidate.struct_name != "Node"),
        "foreign-glob literals are not candidates for the local same-named struct"
    );
}

/// The explicit-import shape: `use bevy_ui::Node;` — provably foreign
/// (the path root is neither crate-internal nor a workspace sibling).
#[test]
fn explicit_foreign_import_literal_is_not_a_candidate() {
    let graph = r"
#[derive(serde::Deserialize)]
pub struct Node { pub id: u32 }
";
    let ui = r"
use bevy_ui::Node;
pub fn panel() { let _n = Node { position_type: PositionType::Absolute }; }
";
    let scan = crate::config_audit::zero_external_read_fields(&[
        ("crates/graph/src/types.rs".to_string(), graph.to_string()),
        ("crates/showcase/src/ui.rs".to_string(), ui.to_string()),
    ]);
    assert!(
        scan.fields
            .iter()
            .all(|f| f.candidate.struct_name != "Node"),
        "explicit foreign imports shadow the sibling module's same-named struct"
    );
}

/// The workspace-sibling shape (the over-suppression the first cut
/// measured on seal-game-editor — 207 candidates lost): `use
/// editor_core::types::Node;` where the declaration sits at
/// `crates/editor-core/…` INSIDE the scanned set. The dash-normalized
/// path-component table must read the import as local.
#[test]
fn workspace_sibling_import_literal_stays_candidate() {
    let core = r"
#[derive(serde::Deserialize)]
pub struct Node { pub id: u32 }
";
    let data = r"
use editor_core::types::Node;
pub fn row() -> Node { Node { id: 7 } }
";
    let scan = crate::config_audit::zero_external_read_fields(&[
        (
            "crates/editor-core/src/types.rs".to_string(),
            core.to_string(),
        ),
        (
            "crates/editor-data/src/store.rs".to_string(),
            data.to_string(),
        ),
    ]);
    assert!(
        scan.fields
            .iter()
            .any(|f| f.candidate.struct_name == "Node" && f.candidate.field_name == "id"),
        "workspace-sibling imports are local evidence"
    );
}

/// The in-crate import shape stays collected (`use crate::types::Node;`).
#[test]
fn in_crate_import_literal_stays_candidate() {
    let src = r"
use crate::types::Node;
mod types {
    #[derive(serde::Deserialize)]
    pub struct Node { pub id: u32 }
}
pub fn row() -> Node { Node { id: 7 } }
";
    let scan = scan_of(src);
    assert!(
        scan.fields
            .iter()
            .any(|f| f.candidate.struct_name == "Node" && f.candidate.field_name == "id")
    );
}

// ── Issue 109 re-sight: the field-set closure ──

/// The exact riir-shader bevy-0.19 shape (`d8c938b`): the file imports
/// BOTH a foreign glob and a workspace-sibling glob, so
/// [`FileScope::literal_is_foreign`] cannot decide (the documented
/// conservative direction reads the literal local). The field sets
/// close it: bevy's `Node` literal names fields that no local
/// same-named decl has — provably a foreign type under the same name.
#[test]
fn foreign_glob_plus_local_glob_field_mismatch_skipped() {
    let graph = r"
#[derive(serde::Deserialize)]
pub struct Node { pub id: u32 }
";
    let ui = r"
use bevy::prelude::*;
use riir_shader_core::prelude::*;
pub fn panel() {
    let _n = Node {
        position_type: PositionType::Absolute,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        padding: UiRect::all(Val::Px(8.0)),
    };
}
";
    let scan = crate::config_audit::zero_external_read_fields(&[
        (
            "crates/riir-shader-graph/src/types.rs".to_string(),
            graph.to_string(),
        ),
        (
            "crates/riir-shader-showcase/src/ui.rs".to_string(),
            ui.to_string(),
        ),
    ]);
    assert!(
        scan.fields
            .iter()
            .all(|f| f.candidate.struct_name != "Node"),
        "a literal naming fields no local same-named decl has is provably foreign"
    );
}

/// The union direction: two same-named decls whose COMBINED fields
/// contain every literal field → still collected (never rejected on a
/// guess when some local decl could own the literal).
#[test]
fn field_mismatch_union_across_same_named_decls_keeps_evidence() {
    let serde_side = r"
#[derive(serde::Deserialize)]
pub struct Node { pub id: u32 }
";
    let other = r"
pub struct Node { pub flex_direction: u8 }
";
    let ui = r"
pub fn panel() -> Node { Node { flex_direction: 1 } }
";
    let scan = crate::config_audit::zero_external_read_fields(&[
        ("crates/a/src/types.rs".to_string(), serde_side.to_string()),
        ("crates/b/src/other.rs".to_string(), other.to_string()),
        ("crates/c/src/ui.rs".to_string(), ui.to_string()),
    ]);
    assert!(
        scan.fields.iter().any(
            |f| f.candidate.struct_name == "Node" && f.candidate.field_name == "flex_direction"
        ),
        "the field exists on SOME local same-named decl — evidence stays"
    );
}

/// The wire half of the re-sight: a mismatched foreign literal inside a
/// serde fn must neither register wire membership (which would FALSELY
/// CLEAR a local struct's stillborn rows) nor construction sites (the
/// wire-const classification).
#[test]
fn field_mismatch_literal_is_not_wire_evidence() {
    let graph = r"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Node { pub id: u32 }
";
    let ui = r"
use bevy::prelude::*;
use riir_shader_core::prelude::*;
pub fn render(record: &Node, out: &mut Vec<u8>) {
    let _n = Node { position_type: PositionType::Absolute };
    if let Ok(bytes) = serde_json::to_vec(&record) {
        out.extend_from_slice(&bytes);
    }
}
";
    let wire = wire_consumption(&[
        (
            "crates/riir-shader-graph/src/types.rs".to_string(),
            graph.to_string(),
        ),
        (
            "crates/riir-shader-showcase/src/ui.rs".to_string(),
            ui.to_string(),
        ),
    ]);
    assert!(
        !wire.cleared.iter().any(|(ty, _)| ty == "Node"),
        "a provably-foreign literal must not clear the local struct's rows"
    );
    assert!(
        !wire.wire_const.iter().any(|(ty, _)| ty == "Node"),
        "a provably-foreign literal must not register construction sites"
    );
}

/// The end-to-end leg over a REAL throwaway git repository: init →
/// config+consumer → the optimization removes the consumer → the report
/// fires ORPHANED with the perf tag, and the never-read twin reads
/// stillborn. Skips (passes silently) only when git is unavailable.
#[test]
fn real_git_e2e_orphan_lifecycle() {
    if git(None, &["--version"]).is_none_or(|o| !o.status.success()) {
        return; // no git on PATH — the unit arms above carry the classifier
    }
    let dir = std::env::temp_dir().join(format!("orphan-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let run = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .expect("git spawn");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "orphan-e2e@test"]);
    run(&["config", "user.name", "orphan-e2e"]);
    let with_consumer = r"
pub struct BehaviorConfig { pub alarm_radius: f32, pub calm: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0, calm: 1.0 } }
}
pub fn alarm_of(c: &BehaviorConfig) -> f32 { c.alarm_radius }
";
    std::fs::write(dir.join("lib.rs"), with_consumer).unwrap();
    run(&["add", "."]);
    run(&["commit", "-q", "-m", "feat: config with a consumer"]);
    let optimized = r"
pub struct BehaviorConfig { pub alarm_radius: f32, pub calm: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0, calm: 1.0 } }
}
";
    std::fs::write(dir.join("lib.rs"), optimized).unwrap();
    run(&["add", "."]);
    run(&[
        "commit",
        "-q",
        "-m",
        "perf: const-table species behavior values",
    ]);
    let report = report_paths(std::slice::from_ref(&dir), &[1]);
    assert!(
        report.blind.is_none(),
        "a real repo resolves the history leg"
    );
    let alarm = report
        .orphaned
        .iter()
        .find(|r| r.candidate.field_name == "alarm_radius");
    let alarm = alarm.expect("alarm_radius orphaned after the optimization");
    assert!(alarm.likely_by_optimization);
    assert!(
        report
            .stillborn
            .iter()
            .any(|f| f.candidate.field_name == "calm"),
        "calm was never read — stillborn"
    );
    assert!(
        report
            .stillborn
            .iter()
            .all(|f| f.candidate.field_name != "alarm_radius")
    );
    let _ = std::fs::remove_dir_all(&dir);
}
