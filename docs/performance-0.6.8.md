# Overlapping preview Restart — 0.6.8

Warm overlapping Restart startup fell from **3.87 s to 3.05 s (21.2%)**, and
Cargo's reported build phase from **1.66 s to 0.71 s (57.2%)**, across nine
replacements per version. These are pooled medians from three alternating batches
of three replacements each, with separate warmed Cargo caches.

## Change and evidence

Concurrent private source paths previously shared the same application artifact
suffix. Dioxus uses its executable as Cargo's workspace wrapper, and Cargo includes
that wrapper path in workspace artifact hashes. Studio now stages an executable
alias per leased workspace using the template's Rust `WorkspaceLease::stage_executable`.
The alias uses a hard link, or a copy preserving executable permissions across
filesystems. Its path survives clean reuse; active and interrupted leases stay
separate. Staging picks up a replaced source executable before new users start.

The baseline used application suffix `-f30720d2a798a3b7` on both leased paths.
The candidate alternated `-a66fb456bc09d05c` and `-07a80eb61530c2ba`, retaining each
identity on reuse. Both support crates stayed fresh. This is evidence that
application artifacts no longer replace each other across these two paths, while
external dependency builds remain shared. Each private path must first warm its
own application artifacts; this does not duplicate the dependency cache.

The mechanism follows [Cargo's documented workspace-wrapper behavior](https://doc.rust-lang.org/cargo/reference/config.html#buildrustc-workspace-wrapper)
and [Dioxus 0.7.10's compiler setup](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/build/request.rs#L1489).
No application sources, release configuration or framework implementation changed.
Ordinary previews retain their previous behavior. All implementation and checks
are Rust; the UI remains Cranpose.

## Measurements and limits

- Apple M5, macOS 27.0, Rust 1.98.1, Dioxus CLI 0.7.10. Baseline Studio 0.6.7
  `fe231e81c90eb28e41af2db0674e276e7548a34d`; candidate SDK
  `94d9470c9d1de3ba1ec95e280ef37b035e6e1696`. Debug runner, same generated
  `desktop-dev` application profile and fixed counter fixture source path.
- Each variant had its own previously populated Cargo cache and four warmup launches
  before measurement. Batch order: before/after, after/before, before/after.
  Initial launches within measured batches are retained in the data but excluded
  from the overlapping Restart medians.
- No task-owned compilation ran during samples. Other desktop applications, the
  installed demo and unrelated Rust builds remained active. There were 1–6 other
  rustc processes at batch starts; this is not continuous interference monitoring.
  These small samples on a busy machine do not predict cold builds or large projects.
- Startup spans runner spawn through a fresh matching inspector snapshot, including
  100 ms connection polling and 200 ms snapshot polling. The IDE swaps at connection,
  so this is **not exact IDE Restart-action latency**. Median spawn-to-connection was
  3.59 s before and 2.77 s after. Cargo durations come from its rounded log output.
- Startup ranges: 3.28–4.73 s before, 2.95–3.57 s after. Every measured candidate
  workspace was reused. All replacements received a fresh response from the previous
  preview (0.48–0.64 ms candidate), and every owned runner/compiler/application exited.
- No cold-start, live-edit, idle CPU, rendering or shutdown speedup is claimed.
  Earlier exploratory runs, including an ineffective external-wrapper override,
  are retained separately and excluded from these results.

The table shows each batch's median of three overlapping replacements:

| Pair | Before startup (s) | After startup (s) | Before Cargo (s) | After Cargo (s) |
|---|---:|---:|---:|---:|
| 1 | 4.003 | 3.075 | 1.79 | 0.72 |
| 2 | 3.872 | 3.038 | 1.86 | 0.71 |
| 3 | 3.287 | 3.045 | 1.45 | 0.70 |

[Raw measurements, warmups, exploratory runs and binary hashes](restart-measurements-0.6.8.json)
retain every collected result.

## Repeat and regression coverage

Build the baseline and candidate runners, then run the shared Rust `xtask hot-smoke`
with the same `--workspace`, separate `--cache` paths, `--startup-only
--restart-rounds 3 --profile-startup --build-diagnostics --report result.json`.
Warm both paths before collecting alternating batches. Candidate runs add
`--require-isolated-restarts --require-cached-support --require-cached-workspace
--require-cached-dependencies`.

The new CI assertion checks distinct aliases/application suffixes during overlap,
stable identities after reuse, and fresh shared support crates. It rejects missing
or shared identity evidence rather than imposing a noisy time threshold. Existing
old-preview response, process-exit, live-edit and recovery assertions remain.
Rust tests cover ownership, executable refresh, cross-filesystem copy permissions,
and actual execution through the alias. The runner passes RustRover analysis;
the SDK has existing `anyhow::ensure!` resolution errors in RustRover while compiler
checks and Clippy pass.

## Additional reload validation

The shipped SDK pin `2fc831e5480d5803497278c16aa55635ee914eeb` adds bounded observed-text/runtime details to
harness timeout errors; its production alias implementation matches the measured
`94d9470` build.

One local counter run acknowledged generation 2 but timed out waiting for its
expected label. Its original console/compiler logs are retained. The same runner
and workspace then passed four edits plus recovery, followed by 21 consecutive
edits plus recovery with the same PID. A baseline run also passed four edits plus
recovery. The initial failure has not been reproduced or assigned a cause; it is
not evidence of improved live-edit reliability or speed. The added diagnostics
make a recurrence observable. This limitation remains a follow-up investigation.
