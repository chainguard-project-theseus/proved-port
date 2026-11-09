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
export set and the linker map differ.

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

- The fuzz-diff driver checks before fuzzing that every function
  resolves into its own object (`dladdr`), that the two objects differ,
  and that each reports the version it should. A harness that silently
  compares a library with itself passes everything.
- Run every leg on every target architecture natively; a fast path that
  exists on one architecture is a different program on the other.
- Give the fuzz-diff driver a voice per mode: the C library's
  interpreter, its JIT, and the port's answer before any JIT compile.
  pcre3 added a fourth voice for JS6 and a rerun of upstream with
  `PCRE2_NO_AUTO_POSSESS` as a second voice for one ledger class.
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
  fails an unlisted predicate.
- `known_test.c` checks each class accepts its issue's cases and refuses
  agreeing look-alikes.
- Narrow a predicate when review shows it excuses more than its class.

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
