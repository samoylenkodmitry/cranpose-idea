# Linux compiler installation and validation — 0.10.1

The first hot-reload launch on Linux failed with `Text file busy` (`ETXTBSY`)
when validating the downloaded Dioxus executable. The installer still held its
writable file handle. Studio now uses the shared Rust cache SDK's executable
publisher: close the handle, validate, then publish without replacing another
installer's winner. Failed candidates are removed. Only `ExecutableFileBusy`
is retried, within a 500 ms budget, because an unrelated concurrent fork can
briefly retain a writable descriptor until exec. Other validation errors return
immediately.

The SDK tests run actual Rust executables before and after publication. They
cover concurrent installation, failed validation, persistent busy errors and,
on Linux, a descriptor deliberately held open during validation. Subprocess
tests run separately from unit tests that require immediate lock reuse. The
original arrangement exposed a macOS test interaction, reproduced locally;
40 complete cache-suite repetitions passed after separating the test processes.
Twenty Linux publication-suite repetitions passed with bounded busy retries.

Ubuntu now runs the same binary/library live-edit, error recovery, overlapping
Restart, inspector and authoring harness as macOS. Its initial tool cache is
empty, so CI exercises installation. Application release sources and dependencies
are unchanged.

## `samarch` observations

The host runs Linux 7.2.6 CachyOS on an AMD Ryzen 9 3950X, with an NVIDIA RTX 2070
and driver 615.71.09. Tests explicitly selected Rust 1.98.1 and two Cargo build
jobs. The online CPU set changed between 0–3 and 0–7 during the session; other
desktop applications remained active. These are functional observations, not a
controlled performance comparison.

- The exact verified 0.10.0 Linux renderer passed lightning endpoint,
  transparency, inline-color, pointer, slider and popup-reuse checks at 1×/2×.
  NVIDIA's process monitor confirmed hardware GPU use.
- With the release measurement harness, first painted frame receipt was
  100.3/61.0 ms at 1×/2× and final transparency was 920.3/899.0 ms. These timings
  exclude the IDE and physical display. The shader emitted zero frames during
  the following 3.03-second settled observation.
- The 1,000-node inspector composed 139 UI nodes and passed scrolling, filtering,
  collapse/expand, details and selection checks. It emitted zero settled frames
  in each five-second visible/hidden observation. Linux CPU samples have one-second
  resolution; zero samples do not establish zero CPU use.
- All nine headless integration groups passed inside the installed IDEA 2026.3
  EAP (263.5153.40), in an isolated profile. This was actual IDE API integration,
  not an interactive Linux desktop session.
- After fixing installation, a binary app passed live values, saved edits,
  compiler-error/syntax/state-shape recovery and state preservation. Its runner,
  compiler and app all exited; observed shutdown was 171 ms.

## Host linker limitation

A subsequent warm launch crashed in the linker with `realloc(): invalid pointer`.
An ordinary Rust test link also crashed; its stack identifies the Rust 1.98.1
bundled `rust-lld`/LLVM 22. An unchanged retry of that test link succeeded.

An empty Dioxus archive initially looked relevant, but two overlapping Restart
replacements passed with the same empty archive and without a linker adapter
when the test process used `taskset -c 0,1`. The cause of the LLVM crash is not
established. No archive filter, linker replacement, system setting or CPU-affinity
policy is included in the plugin change. Later diagnostic runs record their
fixed-affinity condition explicitly; their timings cannot be compared with
unrestricted runs.
