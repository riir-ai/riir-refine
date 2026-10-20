# Research 298 — the uzu `e744e7c..2b4c1ae` window residue: the residency-set scope split (PR #988)

**Status:** RECORD — window fully consumed; one count-neutral rider landed (Plan 266 / Batch 262)

## The window

`e744e7c..2b4c1ae` = 5 commits (walking newest-first: `2b4c1ae` Release 0.6.10 ·
`0b5f991` #993 xgrammar-rs rewrite · `2fa2d06` #992 sysinfo ios workaround ·
`56dbb91` #991 DFlash-TfM cleanup + radix top-k race fix · `e6399cb` #988
residency-set removal). B260 (Plan 264 / Research 296) mined `56dbb91`'s kernel;
B261 set the envelope pin at `2b4c1ae4`; the tail — `e6399cb` — is this record's
subject. Screens NULL: `af697cb` (#977 workspace unify), `2fa2d06`, `0b5f991`,
`2b4c1ae`.

## The atom — PR #988 "Perf: Fix sparse queue residency stalls"

The diff is one line — `sparse_queue.add_residency_set(&residency_set)` deleted
from the Metal 4 context builder — but the PR body carries a complete measured
mechanism with sampled driver stacks (CC-Yeh, merged 2026-10-19):

- **The symptom:** a ~3-second prefill stall on base M5.
- **The law, verbatim:** "Unmap/event batches unnecessarily prepared model
  resources. With insufficient wired-memory headroom, `mach_vm_wire_level_monitor`
  checks failed, triggering costly GPU mapping eviction and rebuilding."
- **The two failure stacks (verbatim, sampled):**
  - *Eviction:* `IOGPUCommandDescriptor::prepare → IOGPUScheduler::freeToWireSysMemory
    → IOGPUMemory::evict → release_mappings → AGXSecureGart::unmapFromThisGart →
    pmap_iommu_unmap`
  - *Preparation:* `IOGPUCommandDescriptor::prepare → IOGPUMemory::prepare_memory_slow
    → commit_mappings → AGXSecureGart::mapInThisGart → pmap_iommu_map`
- **The scope split, verbatim:** "The compute queue keeps model residency for
  prefill/decode. Sparse operations, events, and resource lifetimes are unchanged."

## The law (the rider landed on the gpu_101 host)

A residency set is a **per-queue working-set contract**: every queue wired to a
set promises its command batches the set's whole footprint, and when wired-memory
headroom is insufficient the driver EVICTS and REBUILDS GPU mappings to honor
the promise — so a queue whose work never touches the model buffers (uzu's
sparse queue) must NOT carry the model's residency set, or every unmap/event
batch on that queue pays the wire-monitor's eviction/rebuild cycle. The fix
drops the set from the mis-scoped queue and the compute queue keeps it; the
set object itself is untouched.

**Adjudication as a corpus face:** the gpu_101 host (`residency-set-holding-bound-by-ram-fraction`)
owns the HOLDING BOUND — how much a set may hold (RAM fraction) and the
join/leave discipline. This face owns a different axis: **per-queue SET SCOPE**
— which queues join a set at all. Distinct decision, same family; landed
count-neutral on the host with the driver stacks as the evidence (the host's
own evidence form is a measured revert — this PR is the same genre: a
measured stall, a one-line scope repair).

**Kinship note (recorded, not landed):** the placement-regime analogy to the
mlx-serve prework rule (gpu_97: compute-per-consumer-scope vs produce-once) is
structural, not mechanistic — the prework law prices RECOMPUTE vs share; #988
prices a WORKING-SET PROMISE vs the driver's enforcement of it. The transfer
is the scoping question, not the mechanism.

**Lane intel for our stack:** no current uzu/MTL4 surface (our Metal lane is
CubeCL-emitted, no residency sets); the law transfers to any future MTL4 /
residency-set adoption in riir-ai (the M3 lane) — file this research note as
the pointer.

## Verification

- Vendor clone `E:/git/vendor/uzu` fetched 2026-10-20, HEAD `e744e7c` (behind
  origin by the window — the pinned posture, deliberate).
- The 1-line diff read at `e6399cb`; the surviving compute-queue
  `add_residency_set` read at `2b4c1ae` context.rs; the PR body fetched from
  github.com/trymirai/uzu/pull/988 this session (quotes verbatim above).
- The corpus rider's needle gate asserts the PR-title phrase, the two stack
  heads, and the scope-split sentence.

## Batch 262 disposition

+0 own rules, +1 count-neutral rider (gpu_101 host), corpus stands 923.
Gates: needle gate FIRST-RUN PASS; kernel_opt lib 3176/0; default lib 2702/0;
clippy −D ×3 postures clean. Freeze/score-bench NOT owed (zero clippy-corpus
delta); retrieval floors HELD no re-pin (one rider face on an existing rule —
the B246/B250-slice-2 precedent); binary rebuild NOT owed (kernel_opt outside
the release set).
