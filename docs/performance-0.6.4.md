# Preview startup measurements — 0.6.4

A private dependency-lock cache reduced median warm preview startup from 5.28 s
to 3.59 s in five alternating local pairs (32.0%). The main change was Dioxus's
pre-build preparation: its serving timestamp moved from a median 2.10 s to 0.27 s.
Cargo's reported build phase went from 1.54 s to 1.57 s, so this result does not
show faster compilation or faster live edits.

Each preview starts in a new private directory. Previously it lost the lockfile
that Cargo generated for the instrumented development dependency graph. The runner
now restores a matching private lockfile, lets Cargo validate/update it normally,
and saves it after compiler shutdown. Manifest, original lock, toolchain and Cargo
configuration inputs key the cache. Missing or damaged records fall back to normal
resolution. Application sources and release builds are unchanged.

## Conditions

- Apple M5, macOS 27.0, Rust 1.98.1, Dioxus CLI 0.7.10.
- Isolated counter fixture, shared warm target/helper cache. The new dependency
  cache was warmed once before measurements. No local builds ran during samples.
- Baseline: 0.6.3 plus optional startup instrumentation. Candidate: the same runner
  plus the dependency-lock cache. The order alternated before/after, after/before.
- The user's existing preview remained running. The desktop and network were not
  isolated, so these samples describe this workload, not a universal speedup.
- All ten runs verified both support crates stayed fresh and all three owned
  processes exited. Every candidate launch restored the private lockfile.
- Startup ends at the first matching inspector snapshot. Connection polling adds
  up to 100 ms; snapshot polling adds about 200 ms. Dioxus timestamps have their
  own compiler-process clock and approximately 10 ms display precision.

| Pair | Before startup (s) | After startup (s) | Before compiler serving (s) | After compiler serving (s) |
|---|---:|---:|---:|---:|
| 1 | 7.558 | 3.590 | 3.73 | 0.23 |
| 2 | 5.419 | 3.478 | 2.25 | 0.27 |
| 3 | 4.004 | 3.799 | 0.84 | 0.27 |
| 4 | 5.275 | 3.587 | 2.10 | 0.27 |
| 5 | 4.107 | 3.489 | 0.79 | 0.28 |

[Raw startup samples](startup-measurements-0.6.4.json) retain host, runner, compiler
and Cargo timings. The shared Rust harness records these timestamps directly.
An earlier five-pair batch measured 5.33 s → 4.29 s (19.6%) before the Windows
publication handling was refined; those samples are retained in the raw report.
Cargo cache and network variation differ between batches, so the final comparison
uses only the five adjacent pairs above.

The first uncached launch still resolves dependencies. This change deliberately
keeps Cargo's ordinary online behavior, including validation of changed external
path dependencies. It does not force offline or locked mode.

## Settled idle

Longer idle results are recorded separately with the same isolated fixture. Each
visible, inspecting and hidden phase settles for five seconds and then measures
20 seconds of process CPU. The inspector requests a snapshot every 500 ms.
CPU percentages refer to one core and use a 10 ms CPU clock on this host.

| Run | Visible app CPU | Inspecting app CPU | Hidden app CPU |
|---|---:|---:|---:|
| 1 | 0.847% | 0.697% | 0.000% |
| 2 | 0.697% | 0.548% | 0.000% |
| 3 | 0.448% | 0.598% | 0.000% |

[Raw idle samples](idle-measurements-0.6.4.json) include all three processes. No
frames were rendered during the settled phases. Each inspecting phase sent 40
requests. The compiler recorded no CPU ticks; runner CPU was at most one tick
(about 0.05% of one core). Hidden application CPU recorded zero ticks, which means
below this measurement's resolution, not literally no work.

One earlier baseline observation was 0.797% visible, 0.947% inspecting and zero
ticks hidden. The small differences between these samples do not establish an
idle improvement. This iteration improves warm startup and measurement quality.
