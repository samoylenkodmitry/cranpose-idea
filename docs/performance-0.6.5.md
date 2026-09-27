# Warm preview restart measurements — 0.6.5

Keeping a private workspace path after verified clean shutdown reduced median warm
preview startup from **3.19 s to 2.37 s (25.9%)** in five alternating local pairs.
Cargo's reported build phase fell from **1.33 s to 0.60 s (54.9%)**.

Dioxus 0.7.10 deliberately rebuilds the application crate to capture its linker
invocation for hot reload. A new private source path on every preview prevented
Rust from reusing some incremental compilation work. The runner now leases a stable
path, clears its old contents and instruments a fresh copy of the current sources.
The compiler still performs its normal dependency, source and linker checks.
Its per-launch linker/session cache remains separate.

## Ownership and scope

The template's Rust cache SDK owns the directory lease. An OS lock excludes active
owners, and only an explicit clean-shutdown marker allows reuse. A killed owner
leaves a busy marker even after the OS releases its lock. Concurrent previews get
different private paths. Interrupted directories stay abandoned until the cache is
cleared with all previews stopped.

Before releasing the lease, the runner waits up to another 200 ms for its process
scope to become empty, excluding the runner itself when it owns the shared Unix
group. The SDK observes process groups on macOS/Linux and Jobs on Windows. An
unconfirmed exit abandons the slot. Tests cover live descendants after parent exit,
shared groups, concurrent subprocess leases and interrupted owners.

All mechanics remain in the Rust plugin/template. The original application's
sources and release build configuration are outside this cache. Ordinary previews
keep their previous fresh-directory behavior.

## Conditions and results

- Apple M5, macOS 27.0, Rust 1.98.1, Dioxus CLI 0.7.10.
- Baseline: Studio 0.6.4. Candidate: the same runner with leased workspaces and the
  final process-scope check, using template commit `0bd09743bf069af660893bca75dd1a0c831247b9`.
  The shipped SDK pin `86828a7b09a1d30006c07433a441526b8838d8e4` changes only the
  Windows regression fixture; production code is identical.
- Both versions used the same fixed counter fixture source directory. Each had its
  own Cargo/helper cache, warmed twice before measurement. Order alternated
  baseline/candidate and candidate/baseline. No local builds ran during samples.
- The old development preview was stopped. The desktop and network were not
  isolated. These small-fixture warm results do not predict cold builds or large
  projects, and are not evidence of a live-edit latency improvement.
- Every candidate launch verified workspace reuse. All ten runs restored the
  dependency lock, kept both support crates fresh and verified all three processes
  exited. Candidate shutdowns were 38–150 ms in this batch; no shutdown speedup is claimed.
- Startup ends at a matching inspector snapshot, with 100 ms connection polling
  and 200 ms snapshot polling. Cargo's rounded log reports its build phase.

| Pair | Baseline startup (s) | Candidate startup (s) | Baseline Cargo (s) | Candidate Cargo (s) |
|---|---:|---:|---:|---:|
| 1 | 3.201 | 2.451 | 1.33 | 0.60 |
| 2 | 3.094 | 2.361 | 1.31 | 0.60 |
| 3 | 3.190 | 2.365 | 1.36 | 0.60 |
| 4 | 3.286 | 2.568 | 1.45 | 0.62 |
| 5 | 3.075 | 2.358 | 1.32 | 0.61 |

[Raw samples and warmups](startup-measurements-0.6.5.json) include phase timings,
cache evidence and stopped PIDs. Earlier exploratory pairs shared one Cargo output
directory, allowing the baseline to replace the candidate's incremental artifacts.
They remain in the raw report, with that limitation, and are excluded above.

## Repeat

Use the Rust `xtask hot-smoke` harness with `--startup-only --profile-startup
--build-diagnostics --report result.json`. Keep the same `--workspace` source path
between launches, or use `--fixture counter --reuse-fixture`. Give each compared
runner a separate `--cache` and warm it before measuring. Add
`--require-cached-workspace --require-cached-dependencies --require-cached-support`
to candidate measurements. CI repeats a reusable fixture and requires this evidence
on its second launch, in addition to live-edit state and error-recovery checks.

## Live reload and idle validation

The counter and library-plus-binary fixtures passed remembered-state, stable-PID,
compiler-error, syntax-error and state-type-change recovery checks. The counter's
five additional edits acknowledged in 656–706 ms after the initial 1.39 s patch,
while the harness generated ignored build-output noise. The library fixture's
additional edits ranged from 0.85 to 1.28 s. These are validation samples, without
a matched baseline; no live-edit speedup is claimed.

One counter idle run used five seconds of settling followed by 20 seconds per
phase. Application CPU was 0.85% of one core visible, 1.00% while requesting
inspection every 500 ms, and zero recorded ticks hidden. No frames were emitted
in those settled phases. The compiler recorded zero ticks and the runner recorded
at most one 10 ms tick. This is a limited idle observation, not an idle improvement
or a measure of the entire IDE. All preview processes exited after each run.
