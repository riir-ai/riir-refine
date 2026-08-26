# Dockerfile detection inventory — hadolint + `docker build --check` (dedup ledger input)

**Status:** RECORD — lane-1 input for the Dockerfile heal domain's do-not-reimplement dedup. Sources fetched live 2026-08-26.

**Purpose.** The Dockerfile heal domain must NOT reimplement detection that existing
tools own. Our delta is BOUNDED AUTOFIXES + a retrieval corpus. This note is the
complete detection inventory of both incumbent tools, so rule candidates dedup
against it.

**Sources (fetched, not recalled):**
- hadolint README rule table: https://raw.githubusercontent.com/hadolint/hadolint/master/README.md
- hadolint rule-module listing (authoritative existence check): https://github.com/hadolint/hadolint/tree/master/src/Hadolint/Rule
- Docker build checks reference (21 checks): https://docs.docker.com/reference/build-checks/
- Docker checks how-to (directives, no fix): https://docs.docker.com/build/checks/

---

## 1. Autofix status — THE FACT (pinned)

**Both tools are detection-only. Neither has any fix capability.**

- **hadolint**: the complete CLI flag list in the README (`hadolint --help`) contains
  no `--fix`/autofix flag — only severity/ignore/format/report options. A GitHub
  issues title search for `"--fix"` in hadolint/hadolint returns 4 hits, ALL
  bug-report phrasing ("Fix `Index too large` crash", "How to fix DL4006 warning"),
  none a fix-mode feature request; a search for `autofix` returns **0 issues**.
  Detection-only, verified. (https://github.com/hadolint/hadolint — CLI section;
  https://github.com/search?q=repo%3Ahadolint%2Fhadolint+%22--fix%22+in%3Atitle&type=issues)
- **docker build --check** (BuildKit ≥0.15 / Dockerfile 1.8): the docs describe
  check output as warnings (exit 0 by default; non-zero under `--check`). The only
  configuration surface is the `# check=` directive with `error=true`,
  `skip=<ids>|all`, and `experimental=<ids>|all` — plus the
  `BUILDKIT_DOCKERFILE_CHECK` build-arg mirror. No fix mode exists or is planned
  in the docs. Detection-only, verified.
  (https://docs.docker.com/build/checks/)

**Corollary:** every rule below with "yes" in the bounded-fix column is OUR
territory — the incumbent detects, nobody fixes.

## 2. Rule-count ground truth

- hadolint ships **71 DL rules** (verified against the `src/Hadolint/Rule/` module
  listing on master): DL1001; DL3000–3004; DL3006–3016; DL3018–3030; DL3032–3038;
  DL3040–3067; DL4000–4001; DL4003–4006. **DL3005, DL3017, DL3031, DL3039 do not
  exist in source** — the apt/apk/yum/dnf "no upgrade" family was removed (the
  wiki pages are stale residue; rationale controversial per issue #562). Only
  DL3035 (`zypper dist-upgrade`) survives from that family.
- hadolint ALSO embeds **ShellCheck** for RUN bodies (~34 common SC rules listed
  in README; ShellCheck itself has hundreds). ShellCheck also has NO autofix.
- BuildKit ships **21 build checks** (19 stable + 2 experimental:
  CopyIgnoredFile, InvalidDefinitionDescription).

## 3. Ledger — hadolint DL rules (71)

Category: size / perf / sec / style / runtime / repro. "Bounded fix?" = could a
parse-level proposer emit a correct fix without external knowledge (versions,
registries, intent)?

| Rule | One-liner | Cat | Bounded fix? |
|---|---|---|---|
| DL1001 | inline `# hadolint ignore` pragma used | style | no (user intent) |
| DL3000 | Use absolute WORKDIR | runtime | **yes** — prepend `/` |
| DL3001 | pointless-in-container cmds (ssh, vim, top, ps…) | style | no (semantic removal) |
| DL3002 | Last user should not be root | sec | no (needs a user to exist; policy) |
| DL3003 | Use WORKDIR to switch directory (not `cd`) | style | partial — `cd X && cmd` → WORKDIR + RUN (changes layering) |
| DL3004 | Do not use sudo | sec/runtime | partial — strip `sudo ` prefix (safe only when already root) |
| DL3006 | Always tag the image version | repro/sec | no (needs version choice) |
| DL3007 | Do not use `latest` | repro/sec | no (needs version choice) |
| DL3008 | Pin versions in `apt-get install` | repro | no (needs apt version resolution) |
| DL3009 | Delete apt lists after install | size | **yes** — append `&& rm -rf /var/lib/apt/lists/*` |
| DL3010 | Use ADD for extracting archives | size | partial — ADD↔COPY swap when arg looks like tar |
| DL3011 | Valid UNIX port range 0–65535 | runtime | no (data error) |
| DL3012 | Multiple HEALTHCHECK | runtime | partial — keep last, delete rest |
| DL3013 | Pin versions in pip | repro | no |
| DL3014 | Use `-y` switch (apt-get) | runtime | **yes** — insert `-y` |
| DL3015 | `--no-install-recommends` | size | **yes** — insert flag |
| DL3016 | Pin versions in npm | repro | no |
| DL3018 | Pin versions in `apk add` | repro | no |
| DL3019 | `apk add --no-cache` | size | **yes** — insert flag (then DL3019's own cache-clean becomes moot) |
| DL3020 | Use COPY instead of ADD for files/folders | size/style | **yes** — keyword swap |
| DL3021 | COPY >2 args needs trailing `/` on dest | runtime | **yes** — append `/` |
| DL3022 | `COPY --from` must reference earlier stage | runtime | no (ordering restructure) |
| DL3023 | `COPY --from` cannot self-reference | runtime | no |
| DL3024 | Stage names must be unique | runtime/style | partial — suffix rename (must fix all refs) |
| DL3025 | JSON notation for CMD/ENTRYPOINT | runtime | **yes** — exec-form rewrite (tokenize; quoting edge cases) |
| DL3026 | Only allowed registries in FROM | sec | no (policy config) |
| DL3027 | `apt-get` not `apt` | style | **yes** — keyword swap |
| DL3028 | Pin versions in `gem install` | repro | no |
| DL3029 | Do not use `--platform` with FROM | portability | **yes** — remove flag |
| DL3030 | `yum install -y` | runtime | **yes** — insert `-y` |
| DL3032 | `yum clean all` missing | size | **yes** — append `&& yum clean all` |
| DL3033 | Pin versions in yum | repro | no |
| DL3034 | `zypper install -y` | runtime | **yes** — insert `-y` |
| DL3035 | No `zypper dist-upgrade` | size/repro | no (semantic removal) |
| DL3036 | `zypper clean` missing | size | **yes** — append |
| DL3037 | Pin versions in zypper | repro | no |
| DL3038 | `dnf install -y` | runtime | **yes** — insert `-y` |
| DL3040 | `dnf clean all` missing | size | **yes** — append |
| DL3041 | Pin versions in dnf | repro | no |
| DL3042 | pip `--no-cache-dir` | size | **yes** — insert flag |
| DL3043 | No ONBUILD/FROM/MAINTAINER inside ONBUILD | runtime | no |
| DL3044 | No ENV self-reference within same ENV | runtime | **yes** — split into sequential ENVs (order-preserving) |
| DL3045 | COPY to relative dest without WORKDIR | style | partial — absolutize dest or emit WORKDIR first |
| DL3046 | `useradd` without `-l` (big /var/log/lastlog) | size | **yes** — insert `-l` |
| DL3047 | `wget` without `--progress` (bloated logs) | size | **yes** — insert flag |
| DL3048 | Invalid label key | style | no |
| DL3049 | Label `<x>` missing (schema) | style/meta | partial — scaffold LABEL (needs schema) |
| DL3050 | Superfluous label(s) present | style/meta | partial — delete |
| DL3051 | Label is empty | style | no |
| DL3052 | Label not valid URL | style | no |
| DL3053 | Label not RFC3339 time | style | no |
| DL3054 | Label not valid SPDX | style | no |
| DL3055 | Label not valid git hash | style | no |
| DL3056 | Label not semver | style | no |
| DL3057 | HEALTHCHECK instruction missing (default sev: Ignore) | runtime | partial — insert `HEALTHCHECK NONE` / scaffold |
| DL3059 | Multiple consecutive RUN — consolidate | size(legacy) | partial — `&&` merge. **⚠ ANTI-MODERN premise: hurts BuildKit layer caching; do NOT dedup this into our corpus** |
| DL3060 | `yarn cache clean` missing | size | **yes** — append |
| DL3061 | Dockerfile must begin with FROM/ARG/comment | runtime | partial — reorder |
| DL3062 | Pin versions in `go install` (`@version`) | repro | no |
| DL3063 | Stage name not a reserved word | runtime/style | partial — rename + refs |
| DL3064 | Sensitive data in ARG/ENV | sec | no (needs secret-mount rewrite — structural) |
| DL3065 | `FROM --platform=$TARGETPLATFORM` redundant | style | **yes** — remove |
| DL3066 | Non-numeric UID may not resolve | runtime | no |
| DL3067 | Do not copy entire filesystem from another stage | size | partial (needs selection knowledge) |
| DL4000 | MAINTAINER is deprecated | style | **yes** — rewrite to `LABEL maintainer=...` |
| DL4001 | Use wget OR curl, not both | size | partial — remove one (semantic choice) |
| DL4003 | Multiple CMD | runtime | partial — keep last |
| DL4004 | Multiple ENTRYPOINT | runtime | partial — keep last |
| DL4005 | Use SHELL to change default shell | style/runtime | partial — insert SHELL instr |
| DL4006 | Set `-o pipefail` before RUN with pipe | runtime | **yes** — insert `set -o pipefail;` prefix or SHELL opts |

hadolint also runs **ShellCheck (SC####)** inside RUN bodies (SC1000–SC2164 in
the README's common list — quoting, word-splitting, useless cat, cd||exit, etc.).
SC = ShellCheck's territory, detection-only there too; our Dockerfile domain
should consume/ignore SC spans, not reimplement shell analysis.

## 4. Ledger — BuildKit `docker build --check` (21)

| Check | One-liner | Cat | Bounded fix? |
|---|---|---|---|
| StageNameCasing | stage names lowercase | style | **yes** — lowercase (fix refs) |
| FromAsCasing | `as` case matches `FROM` | style | **yes** — case-align token |
| NoEmptyContinuation | no empty `\` continuation lines | style/runtime | **yes** — delete blank line (future error) |
| ConsistentInstructionCasing | uniform instruction casing | style | **yes** — normalize to upper |
| DuplicateStageName | stage names unique | runtime | partial — rename + refs |
| ReservedStageName | no reserved words as stage names | runtime | partial — rename + refs |
| JSONArgsRecommended | exec form for CMD/ENTRYPOINT (signals) | runtime | **yes** — exec-form rewrite (= DL3025) |
| MaintainerDeprecated | LABEL instead of MAINTAINER | style | **yes** (= DL4000) |
| UndefinedArgInFrom | FROM must use declared ARGs | runtime | partial — insert `ARG` line |
| WorkdirRelativePath | absolute WORKDIR | runtime | **yes** (= DL3000) |
| UndefinedVar | vars defined before use | runtime | no (intent) |
| MultipleInstructionsDisallowed | one HEALTHCHECK/etc per stage | runtime | partial — merge/keep-last |
| LegacyKeyValueFormat | `ENV k=v` not `ENV k v` | style/runtime | **yes** — `=` rewrite |
| RedundantTargetPlatform | drop `--platform=$TARGETPLATFORM` | style | **yes** (= DL3065) |
| SecretsUsedInArgOrEnv | no secrets in ARG/ENV | sec | no (= DL3064) |
| InvalidDefaultArgInFrom | global ARG default → empty base name | runtime | partial |
| FromPlatformFlagConstDisallowed | no constant `--platform` | portability | **yes** — remove flag or var-ify (= DL3029 family) |
| CopyIgnoredFile *(exp)* | COPY of .dockerignore-excluded file | runtime | no (fix context/ignore file) |
| InvalidDefinitionDescription *(exp)* | `# <description>` comment format for stages/args | style | **yes** — comment formatting |
| ExposeProtoCasing | lowercase protocol in EXPOSE | style | **yes** — lowercase |
| ExposeInvalidFormat | no IP:host-port mapping in EXPOSE | runtime | **yes** — strip to port/proto |

Overlaps with hadolint (dedup notes): JSONArgsRecommended≡DL3025,
MaintainerDeprecated≡DL4000, WorkdirRelativePath≡DL3000,
RedundantTargetPlatform≡DL3065, FromPlatformFlagConstDisallowed⊃DL3029,
SecretsUsedInArgOrEnv≡DL3064, UndefinedArgInFrom (no DL twin),
DuplicateStageName≡DL3024, ReservedStageName≡DL3063.

## 5. Open space — NOT detected by either tool (verified against both lists)

Each verified absent from the 71 DL rules + 21 checks + SC list above:

1. **Cache mounts** (`RUN --mount=type=cache,target=...` for package dirs) — no
   rule detects a missed cache-mount opportunity. **OPEN.** Bounded fix: insert
   the mount flag for known package-manager dirs. Arguably our highest-value
   perf rule (build-speed axis neither tool touches at all).
2. **Dependency-layer ordering** (COPY manifest → deps install → COPY source) —
   no rule checks COPY order for cache efficiency. **OPEN.** (DL3059 pushes the
   *opposite*.) Fix = reorder + split, bounded but structural.
3. **Digest pinning** (`FROM img@sha256:...`) — DL3006/DL3007 stop at tags;
   nothing checks digest absence. **OPEN** for detection; the *fix* needs a
   registry lookup (L4/healer territory, not parse-level).
4. **SBOM / provenance attestations** — absent from both. **OPEN** (fix is
   buildx-flag/syntax-directive territory, not Dockerfile body).
5. **PID1 zombie reaping** (`--init`, tini/dumb-init) — DL3025/JSONArgsRecommended
   cover signal delivery only. **OPEN-partial** (exec form is covered; reaping is not).
6. **Secrets in RUN command lines** (tokens in URLs, `git clone https://token@…`)
   — DL3064/SecretsUsedInArgOrEnv cover ARG/ENV only. **OPEN.**
7. **`--chown` on COPY for non-root files** — no rule. **OPEN** (pairs with DL3002).
8. **.dockerignore absence / context bloat** — CopyIgnoredFile is the inverse
   case. **OPEN.**
9. **Locale/env reproducibility pins** (`ENV LC_ALL=C.UTF-8` etc.) — no rule. **OPEN.**
10. **`# syntax=` directive version pinning** — no rule. **OPEN.**
11. **Final-stage minimality** (small base / scratch final stage) — DL3067 catches
    only whole-FS copies. **OPEN-partial.**
12. **`apt-get update`+`install` same-layer pairing** — approximated by DL3009's
    list-cleanup check only. **Partial coverage.**

**Do-not-dedup warning:** DL3059 (consolidate RUNs) encodes a pre-BuildKit
premise. Our corpus should not adopt it as a fix target for modern targets.

## 6. Verdict for the Dockerfile domain

Detection is saturated (71 DL + 21 checks + ShellCheck); **fixing is completely
unoccupied**. ~25 rules above are parse-level bounded fixes (the `**yes**` rows).
The genuinely novel *detection* territory is small but real: cache mounts,
dep-layer ordering, digest pinning detection, RUN-line secrets, PID1 reaping —
the build-speed/cache axis is the open lane both incumbents ignore.
