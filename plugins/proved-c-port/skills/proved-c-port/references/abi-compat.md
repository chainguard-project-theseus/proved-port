# ABI compatibility: the C library as oracle

The port is correct when it answers as the C library does, except where
a numbered ledger entry says otherwise. Build every comparison so that
both libraries run the same inputs through the same entry points.

## What must match exactly

Write this list down in the design doc (pcre3 design 3.3):

- header constants and macros;
- exported names, sonames, symbol version nodes;
- error codes and error texts;
- public struct layouts (and which structs are opaque);
- argument checking order where callers can observe it (pcre3 restored
  upstream's cap-before-verify order after a review showed a reordered
  check reported `HEAP_FAILED` where upstream reports `SIZE_TOO_BIG`
  under OOM).

Correct: `ci/header-check.sh` diffs every `PCRE2_*` and `REG_*`
definition against the distro's `pcre2-dev`; `ci/abi.sh` fails when the
export set and the linker map differ. A port pinned to one upstream
release compares with that release instead, built from its tarball by
SHA-256 (`references/lifecycle.md`, phase 1).

Wrong: regenerating the header from your own constants, or editing an
upstream version node except to add a new upstream node.

## The comparison legs

| leg | what runs | gate |
|---|---|---|
| conformance | upstream's test data through upstream's driver (patched to the public API only), against the port at every code-unit width, plain and with the fast path | blocking; per-block ratchet, strict both ways |
| differential | the same inputs plus real-world corpora on the port and on the distro's C library | on request (label); advisory ratchet |
| fuzz-diff | one C driver `dlopen`s both libraries with `RTLD_LOCAL`, compares compile, info, match, DFA and substitute results | on request; a divergence not named in `known.c` fails |
| fuzz-abi | upstream's OSS-Fuzz harness under ASan and UBSan, linked against the port, with caller-buffer guard bytes and a tracking allocator | on request; any sanitizer report or internal error fails |
| consumers | real dependents built from pinned sources against the C `-dev` package, their own tests run with the port on `LD_LIBRARY_PATH` | on request; compared with `expected-failures.txt` |

Rules for the legs:

- The fuzz-diff driver proves before fuzzing that each side is the
  library it claims to be, and that the two differ. A harness that
  silently compares a library with itself passes everything. The
  mechanism may vary: pcre3's C driver `dlopen`s both and checks with
  `dladdr` that every function resolves into its own object and that
  each reports its version; expat-rs's Rust fuzzer `dlopen`s the
  reference by path with `RTLD_LOCAL`, checks with `dladdr` that every
  reference function resolves into that file (same device and inode)
  and its own `Parser::parse` into the executable, checks the
  reference's version, and refuses to run while any libexpat is in the
  global scope (`identify` in `fuzz/fuzz_targets/differential.rs`).
  Self-test the check with a plant that points both sides at the same
  library and must be refused, or with a planted reference (expat-rs's
  `fuzz/diff/planted.c`: upstream with one deliberate difference) whose
  run must report a FINDING, which vanishes if both sides are one
  library. A difference planted in the driver's comparison still shows
  when both sides are the reference, so it cannot catch a
  self-comparison.
- fuzz-abi uses upstream's OSS-Fuzz harnesses. When they live in
  google/oss-fuzz (`projects/<name>`) rather than upstream's tree,
  vendor them unedited at a pinned OSS-Fuzz commit, with their own
  licence, and adapt them with a forced include (`-include`) rather than
  patches (chainz #17).
- Stable Rust is not ASan-instrumented: ASan sees the port's bulk copies
  only through libc interceptors (`memcpy`, `memset`). The guard bytes
  around caller buffers are what check the port's own stores; do not
  count on ASan for them.
- Build the fuzz-abi library with `panic = "abort"`, so any panic the
  guard would catch is a crash: a caught panic is a port bug, and a
  port that returns the ordinary failure value for one gives C no other
  way to see it. Check it by the `panic_unwind` crate's symbols, not by
  the absence of `_Unwind_RaiseException` (with `-static-libgcc` the
  unwinder is always linked).
- Run the C tests under ASan and UBSan on every PR when it is cheap
  (expat-rs's `ci/sanitize.sh`); keep the fuzzers on request.
- Run every leg on every target architecture natively; a fast path that
  exists on one architecture is a different program on the other.
- Give the fuzz-diff driver a voice per mode: the C library's
  interpreter, its JIT, and the port's answer before any JIT compile.
  pcre3 added a fourth voice for JS6 and a rerun of upstream with
  `PCRE2_NO_AUTO_POSSESS` as a second voice for one ledger class. A
  fast path the C library shares needs a voice of the port against
  itself without the fast path (`references/lifecycle.md`, phase 5).
- Keep slow legs behind labels. Add the labels before the push that
  should run them (see `references/landing.md`), and drop them before
  the final landing push once they have passed on that head, so the
  landing run is fast.

## The deviations ratchet

The conformance gate is a list of failing blocks, each with a one-line
reason that starts with an active ledger entry. It is checked strictly
both ways: an unlisted failure, a listed block that passes, and a line
without a reason all fail.

- Only remove lines, using the script's `--update` mode, and re-run it
  after a rebase instead of merging by hand.
- A new failure is a regression to fix, never a line to add.

## The known-differences ledger

`docs/abi-changes.md` holds every deliberate, observable difference. A
difference without an entry is a bug. IDs are `ABI-nnn`, assigned in
order and never reused. The format is fixed so an agent can turn an
entry into a distro package patch without other context:

```markdown
### ABI-nnn: one-line summary

- **Status:** active | planned | removed (date, PR #n)
- **Since:** version (PR #n)
- **Area:** compile | match | config | serialize | jit | ...
- **Surface:** the exact functions, macros, options or error codes
- **upstream:** what the C library does
- **port:** what the port does instead
- **Why:** one or two sentences, with a design reference
- **Affected consumers:** from the consumer survey; "none known" otherwise
- **Detection:** a probe, a grep pattern for consumer sources, a test name
- **Patch recipe:** file, function and replacement logic, or "none needed" and why
```

Examples worth copying: ABI-001 (the version string carries the port's
identity so libselinux recompiles instead of decoding a foreign blob),
ABI-002 (serialization stores the pattern and recompiles, so a C-library
stream is refused with `BADMAGIC`).

## `known.c`: one narrow predicate per class

- Each predicate is keyed on what the input contains and which results
  differ, never on a hash of the input.
- Each row cites an active `ABI-nnn` or an open bug (`TODO(#n)`); the
  script fails a row that cites neither, and `-Werror=unused-function`
  fails an unlisted predicate. Check the issue's state too: expat-rs's
  `ci/fuzz-diff.sh --check-known` fails a row citing a closed issue, and
  a `Fixes #n` in the PR that leaves `TODO(#n)` cited.
- `known_test.c` checks each class accepts its issue's cases and refuses
  agreeing look-alikes.
- Narrow a predicate when review shows it excuses more than its class.

Where the surface is small, tolerate no difference at all: no
`known.c`, and for each deliberate difference the driver does not
generate the input class, with a comment citing the ledger entry
(chainz generates only `prime_bits < 8`, skipping the `>= 8` class
UP-001 records). It is stricter, but the
avoided class is then compared by nothing, so keep the list of avoided
classes short and tested elsewhere.

## Caller callbacks and allocators

Function pointers a caller hands in are ABI too: custom allocators
(zlib's `zalloc`/`zfree`/`opaque`, PCRE2's general context, expat's
memory suite) and handlers the library calls back.

- Honour the caller's allocator for everything the C library allocated
  through it, with its alignment, and free through the matching call
  (chainz ABI-006; expat-rs S9).
- A callback may panic (a Rust caller) or throw (a C++ one) through the
  port; test both and say in the ledger what the port does (chainz
  ABI-005, the `panic_in_zfree_*` tests).
- A handler may re-enter the library on the same handle. Decide which
  calls are allowed from inside a callback, refuse the rest, and test
  them (expat-rs S8).

## Upstream bugs

Fuzz-diff finds bugs in the C library too: reference crashes, internal
errors, JIT disagreements with its own interpreter, wrong bounds.

- File them in the port's tracker with an `upstream issue` label, so
  they are told apart from the port's own divergences.
- Keep the issue open until upstream fixes the bug. The port settling
  its side (not copying the bug, a ledger entry, a fuzzer skip) does not
  close it. PRs touching it say `Refs #N`, never `Fixes #N`.
- Do not copy an upstream bug for compatibility unless a consumer needs
  it; when the C library crashes, the port answering is a ledger entry.

## Consumers

- Pick consumers from the survey, covering each API surface and width
  (pcre3: grep, git, glib, nginx, php for 8-bit; qt for 16-bit; fish for
  32-bit; tig for POSIX; rizin for all three at once).
- Build them from pinned upstream sources against the C library's `-dev`
  package, then run their own regex tests with the port's library first
  on the library path. That is the drop-in claim, tested.
- Infra flakes (package DNS lookups) are reruns, not findings; record
  the class once.
- For a `malloc` replacement, also test the as-needed link trap: a
  `pkg-config` file that wraps `-lchalloc` in `--no-as-needed`, because a
  program that names no allocator symbol would otherwise silently run on
  glibc's `malloc`.

## Identity and coexistence

- The port's version string must never equal a real upstream string
  (pcre3 appends `(pcre3 <version>)`), so consumers that cache compiled
  artifacts by version recompile instead of feeding a foreign blob to
  the port.
- Ship a `dlsym` probe so consumers and tests can tell which library is
  loaded (`challoc_version()`).
