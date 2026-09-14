# Issue 109: orphan-report FP refinements — foreign-type struct literals counted stillborn; generic serde bridges defeat the co-occurrence wire-clearance

**Status:** OPEN — instrument intake from the 2026-09-15 quiet-repo sweep (9 repos, `--orphan-report` + `--config-audit`; one filed adjudication record: seal-game-editor `.issues/157`). No repair mandated by this issue — both rows are specimen evidence for the FP-class ledger, filed so the next instrument pass has them without re-deriving.

## Specimen A — foreign-type struct literals (appears unrecorded)

riir-shader `crates/riir-shader-showcase/src/ui.rs:268-273`: four `[stillborn]` rows for `Node.flex_direction` / `Node.padding` / `Node.position_type` / `Node.row_gap`. `Node` is `bevy_ui::Node` — the struct literal's fields are consumed by the Bevy layout engine, outside any scannable set. The tool's own scope footer covers "readers outside the scanned set are invisible", but these rows still **inflate the stillborn count** — the sweep-noise law (a report nobody re-runs because it cries wolf is dead) applies: riir-shader read "4 stillborn" with zero true findings.

Candidate repair: a struct literal naming a type **not declared in the scanned set** is foreign by construction — skip its fields, or annotate them `[foreign-type]` out of the stillborn count the way `[wire-const]` sits outside `[stillborn]`. The declared-type table already exists (the C2 pass needs it to find own-impl reads).

## Specimen B — generic serde bridge (new shape for known FP class #2)

seal-game-editor `crates/feat-item/src/lib.rs:3055`: `fn row_to_mapping<T: serde::Serialize>(row: &T) -> Option<serde_yaml::Mapping>` — the Cluster B lane's edit buffer is built by `serde_yaml::to_value(row)` through this **generic** bridge over `CraftRow` / `ComposeTemplateRow` / `StoneStepRow` / `StoneOptCostRow`, and the same shape feeds the item page's extras `Mapping` for `ItemRow` / `DropTierRow`. All six carry live `updated_at` fields that read `[C1] ORPHANED-BY-TIME`.

The landed fn-scoped co-occurrence repair (`.distill/001` FP class #2, 2026-09-12) cannot fire here by construction: the bridge fn contains **no struct literal** (the type is a generic parameter), and the callers contain **no serde call**. This is a strictly different shape from the documented fn-A/fn-B residual — co-occurrence is not merely missed, it is inapplicable.

Candidate repair (weaker, honest): a field on a struct that derives `Serialize`/`Deserialize` **and** is read via a generic `to_*` bridge anywhere in the scan is at best `[wire-cleared]`-equivalent severity; or, the conservative annotation route — downgrade C1/stillborn rows on Ser+De structs to a weak marker, accepting that a genuinely-dead wire field rides at weak severity too (the trade FP class #2 already weighed and accepted once).

## Sweep context (no other true findings)

riir-chain, riir-game-sdk, seal-remake: 0 rows. riir-dao: the known `[wire-const] EpochSummary.provenance_verified`. riir-deployer `TargetState.started_unix` / riir-auth `AccountState.created_at_unix`: serialized write-only provenance — deliberate, no action. Full adjudication + the seal-game-editor 26-row record: seal-game-editor `.issues/157` (amended 2026-09-15 with the Cluster B / dirty-tracking verification).
