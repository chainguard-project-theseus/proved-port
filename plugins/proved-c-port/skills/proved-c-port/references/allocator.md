# Allocator ports: what changes when the C library is `malloc`

challoc replaces glibc's `malloc` and jemalloc. Most of the skill
applies unchanged; this file covers what is different when the port is
the process allocator, which every other library (including other
ports) runs on.

## The ABI surface is wider than one header

Inventory every family, and test each from C under `LD_PRELOAD`:

- C: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`,
  `aligned_alloc`, `memalign`, `valloc`, `pvalloc`, C23's `free_sized`
  and `free_aligned_sized`.
- glibc extras: `malloc_usable_size`, `malloc_trim`, `mallopt`,
  `mallinfo2`.
- jemalloc's extended API under both plain and `je_` names: `mallocx`,
  `rallocx`, `xallocx`, `sallocx`, `dallocx`, `sdallocx`, `nallocx`,
  `mallctl`, `mallctlnametomib`, `mallctlbymib`, `malloc_stats_print`,
  the `MALLOCX_*` flags, and its `malloc_conf` syntax.
- C++ `operator new` and `operator delete` in every form; the `nothrow`
  forms need one small C++ file, because Rust cannot catch a C++
  exception from a `new_handler`. Check the library never needs
  libstdc++ (`cxx_symbols_in_release_so`).
- Packaging: the port's own soname, plus a compatibility stage that
  installs it under the replaced library's soname and `.pc` file.

Rules:

- Every export is a top-level `pub extern "C"` with `#[unsafe(no_mangle)]`
  or `export_name`.
- Never allocate through anything but the allocator's own machinery,
  including in its own error paths and configuration parsing.
- Being stricter than the replaced allocator is a ledger item: challoc
  aborts on a sized free that names the wrong class, and on
  `malloc_usable_size` of a freed or foreign pointer, where jemalloc's
  behaviour is undefined.

## Thread caches and TLS

- Use one initial-exec TLS slot through an `asm!` accessor, never
  `thread_local!` (which may allocate). Initial-exec covers linking and
  `LD_PRELOAD`; `dlopen` after start is unsupported, and the design says
  so.
- Handle the bootstrap and teardown windows with sentinels in the slot
  (creating, exited): an allocation inside `pthread_setspecific` or from
  another key's destructor after exit is served with no cache.
- Thread exit flushes every cache through a `pthread_key` destructor.
- Ship a debug build that checks every slot-to-heap use against a
  registry of published heaps (challoc's `challoc_tls_check` cfg), and
  run the unit, API and lifecycle tests under it in CI.

## Fork safety

- Register `pthread_atfork` handlers from the library's constructor,
  before the allocator initialises. The prepare handler takes every
  allocator lock with signals blocked; parent and child release.
- Handlers registered before the allocator's run while it holds every
  lock. Serve allocations from that thread inside the fork window
  (treat its lock requests as held) so an early handler that allocates
  does not deadlock.
- Freeze what the handlers snapshot: refuse background-thread start and
  arena creation inside the window with `EAGAIN`.
- Write down the residual: under `LD_PRELOAD`, glibc runs every
  `DT_NEEDED` constructor before the preloaded allocator's, so a
  thread-pool library that registers a waiting handler early still
  deadlocks `fork`. Test each case as a C program (`fork_window.c`,
  `atfork_early.c`, `atfork_pool.c`, `fork_signal.c`).

## Testing against glibc and peers

- Vendor glibc's own malloc tests at a pinned tag and run each program
  twice, as challoc's `glibc!` rows do: bare, where it must fail (a
  control that the row really depends on what is loaded), and under
  `LD_PRELOAD` of the port, where it must pass.
- Vendor mimalloc's stress tests.
- Keep negative tests per hardening property (`tests/c/negative.c`):
  invalid free, double free, interior pointer, foreign pointer.
- Test under memory limits in real containers (cgroup v2 `--memory`),
  with swap off, one container per run.
- Test both size-class tables (challoc's quantum 16 and quantum 8) in
  every leg, Verus included. A function can exceed its rlimit only under
  the other table.
- Test the kernel floor on real old kernels (challoc's AlmaLinux 9 5.14
  cells) and probed features on hosts with and without them. A missing
  feature may change cost, memory or detection, never results.
- challoc's floor cells run postsubmit only, on pushes to main
  (`.chainguard/ci/floor.yaml`): they hold the identity that creates the
  GCE VMs, and that identity is never released to a pull request. Do not
  wait for floor cells on a PR. A change that may need a newer kernel
  says in its body how it keeps the floor, and a floor red on main after
  a merge is that PR's to fix forward.

## Proofs that matter for an allocator

challoc's theorem families, as a checklist:

- Fit and alignment: a non-NULL result has at least the requested
  usable bytes and the quantum's alignment.
- Uniqueness: two live blocks' usable bytes are disjoint and their
  addresses differ (linear `Live` tokens make this structural).
- Metadata integrity: a returned block shares no byte with allocator
  metadata or the shared static.
- Region separation and overflow reach: a 4 MiB never-committed gap lies
  between any block and any other region.
- Valid frees never abort: a free of a live block, through any cache
  path, sized or not, never reaches the invalid-free or double-free
  checks.
- Fast paths make no system call and take no lock, with the hot
  exports' instruction budget pinned in CI (theorem 45, `perf-budget`).
- Size-overflow classes from glibc, jemalloc and TCMalloc CVEs are ruled
  out on the entry points' contracts.

State the C caller's premise once (it frees only live blocks it was
given, once) as a named axiom, and cite it from every theorem that
rests on it.

## Concurrency evidence

Verus does not model weak memory. Layer the evidence and label each
layer's strength in the seam statuses:

- loom models of the atomics protocols, over the real atomic wrappers,
  with pin tests that fail when a copied source line changes (partly
  checked);
- GenMC ports under RC11, IMM and ARMv8, with controls that must fail
  (cited, but moving no seam);
- hardware stress tests on aarch64 for the shipped orderings;
- a check that rustc emits no `noalias` on `&mut Heap` receivers when
  the code writes through raw pointers while one is live
  (`ci/heap-noalias.sh`).

## Performance

- The comparison set is the stock allocators measured in the same run
  on the same VMs: glibc, jemalloc (each relevant release), mimalloc,
  tcmalloc.
- Benchmarks of record are real consumers' soaks (Valkey, Ruby, JVM,
  PostgreSQL), not microbenchmarks; larson counters do not stand in for
  a 550 MiB Valkey working set.
- RSS under a cgroup limit is a first-class row, next to throughput and
  p99.
- A refactor proves "no codegen change" with a machine-code diff on the
  hot set instead of a benchmark.
- Pin `malloc` and `free` entry placement and check it on the shipped
  library (`nm -D`), so alignment does not drift between builds.
