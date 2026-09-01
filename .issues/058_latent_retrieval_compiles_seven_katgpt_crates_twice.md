# 058 — `latent_retrieval` compiles seven katgpt crates twice

> **Status:** **RESOLVED 2026-09-01 — fixed in this commit.** Under
> `latent_retrieval` (and therefore `bonsai_retrieval`) this crate resolved
> **two distinct copies** of seven `katgpt-*` crates — one from the local
> checkout, one from git at a revision four commits older. A four-line
> `[patch]` table in the root manifest collapses them; 526 → **519** packages.
>
> **Validation:** `cargo check --offline --locked` **exit 0** on the default
> feature set AND on `--features latent_retrieval`; no `warning: patch … was
> not used` in either; `grep '^name = ' Cargo.lock | … | uniq -d` now returns
> nothing. Regression-gated by **C7** in
> `../riir-ai/scripts/ci_boundary_contract.sh` (`riir-ai/.issues/840`), which
> reported all seven before the fix and is green after.
>
> ⚠️ **`Cargo.lock` is gitignored here** (`.gitignore:2`), so CI resolved this
> fresh on every run — the duplication was manifest-derived, not a stale-lock
> artifact, and could not have been "fixed" by regenerating a lockfile.
>
> Retained rather than removed (contra the noise-reduction rule) because the
> `[patch]` table in `Cargo.toml` cites it: a future reader who deletes that
> table as redundant needs to find this.

## What was measured

`Cargo.lock` carries seven `katgpt-*` names twice — same version, different
source:

| crate | copy A | copy B |
|---|---|---|
| `katgpt-core` 0.4.1 | local path | git `14116501` |
| `katgpt-dec` 0.1.3 | local path | git `14116501` |
| `katgpt-hla` 0.1.3 | local path | git `14116501` |
| `katgpt-micro-belief` 0.2.1 | local path | git `14116501` |
| `katgpt-personality` 0.1.3 | local path | git `14116501` |
| `katgpt-sense` 0.2.1 | local path | git `14116501` |
| `katgpt-types` 0.2.1 | local path | git `14116501` |

`cargo tree` refuses the bare name outright, which is the shortest proof:

```
$ cargo tree --features latent_retrieval -i katgpt-core
error: specification `katgpt-core` is ambiguous
  path+file:///Users/katopz/git/katgpt-rs/crates/katgpt-core#0.4.1
  git+https://github.com/katopz/katgpt-rs?branch=main#katgpt-core@0.4.1
```

The second copy arrives through `riir-neuron-db`:

```
katgpt-core v0.4.1 (github.com/katopz/katgpt-rs?branch=main#14116501)
└── riir-neuron-db v0.1.1
    ├── neuron-db-sdk → riir-rag → riir-clippy
    ├── riir-clippy
    └── riir-rag
```

## Why

`riir-neuron-db` deps katgpt by **git URL** and redirects it to the local
checkout with `[patch]` (`riir-neuron-db/Cargo.toml:1749`). **`[patch]` is
honoured only in the root manifest of the workspace being built.** When
`riir-clippy` is the build root, that redirect does not apply, so
`riir-neuron-db`'s katgpt edge stays on git — while this crate's own
`Cargo.toml:15` path dep resolves to the local checkout. Two roots for the same
crate, so cargo compiles both.

`riir-ai`, `riir-game-sdk`, `riir-mmorpg-examples` and `riir-neuron-db` all
carry the patch table. This repo does not. **It is the only repo in the
workspace with duplicated workspace-owned crates** — checked across every repo's
lockfile for any name starting `katgpt-` / `riir-` / `neuron-db` appearing more
than once; riir-clippy is the sole hit.

## Cost

- **Seven redundant crate compilations**, `katgpt-core` among them, in a build
  path CI exercises in **release**:
  `.github/workflows/ops_dashboard.yml:275` —
  `cargo build --release --features latent_retrieval,rust_perf --bin cargo-heal`.
  `scripts/self_evolve_audit.sh` needs the same build.
- **A latent type-unification trap.** The two copies are the same name at the
  same version but distinct crates, so their types do not unify. Any API that
  passes a `katgpt-core` type between this crate's own code and
  `riir-rag`/`riir-neuron-db` fails with `expected katgpt_core::X, found
  katgpt_core::X` — an error that reads like a compiler bug. Nothing hits it
  today; the seam simply has not been crossed.
- **Stale shared fixtures on one side.** `14116501` (2026-08-24) sits **four
  commits before** `6f392727`, and that window contains `roll_die v2 —
  rejection-test every draw (exact uniformity)` plus a substantial rewrite of
  `katgpt-device-verify`'s pinned vectors (`vectors.rs` +23,
  `vectors_data.rs` +208/−112). Those vectors are precisely the drift detector
  katgpt-rs's own contract describes — *"each asserts the same pinned vector
  fixture by value … copying the fixture is what detects it."* Here both
  versions live inside **one** build, where no fixture comparison can see them.

⚠️ Not a correctness defect today: `katgpt-device-verify` resolves to the git
copy only, so there is no split-brain within this build, and nothing in this
repo asserts those fixtures (`grep katgpt_device_verify` over `*.rs`: **0**
files). The hazard is that the duplication is silent and nothing detects it.

## Remedy (compile-verified, not yet applied)

```toml
# `[patch]` is honoured only in a workspace ROOT. `riir-neuron-db` deps katgpt
# by git URL and patches it back to the local checkout in ITS root — a redirect
# that does not reach us when riir-clippy is the root, leaving a SECOND copy of
# seven katgpt crates in the graph under `latent_retrieval` (issue 058).
[patch."https://github.com/katopz/katgpt-rs"]
katgpt-core          = { path = "../katgpt-rs/crates/katgpt-core" }
katgpt-device-verify = { path = "../katgpt-rs/crates/katgpt-device-verify" }
```

Verified 2026-09-01 by injecting the equivalent with `cargo check --config`
(no tracked file modified, `Cargo.lock` backed up and restored):

- **baseline** `cargo check --offline --locked --features latent_retrieval` → **exit 0**
- **patched** same command with the two `--config patch…` entries → **exit 0**,
  and `cargo tree -i katgpt-core` now resolves unambiguously with
  `riir-clippy` and `riir-neuron-db` sharing one local copy.

That the patched build compiles is expected rather than lucky: it puts
`riir-neuron-db` on the same katgpt revision it already builds against
natively in its own repo.

`katgpt-attn-match` is deliberately omitted — it is optional and not enabled on
this path, so patching it emits `warning: patch … was not used in the crate
graph`. Add it only if that changes.

## Follow-up worth taking

Nothing detects this class. A lockfile check — *no workspace-owned crate name
may appear twice in any repo's `Cargo.lock`* — is cheap and, measured across
the workspace today, has exactly one hit and no false positives. The natural
home is the workspace contract guard
(`riir-ai/scripts/ci_boundary_contract.sh`), which already reasons about
cross-repo edges but sees only `path =` deps and so is structurally blind to a
git-sourced duplicate. Filed as `riir-ai/.issues/840`.
