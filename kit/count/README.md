# verus-count

The coverage counter of a proved port: it reads the port's sources with
`verus_syn`, classes every executable function and its lines as
verified, trusted, discharged or outside `verus!`, and holds the shipped
crates' shares to `verus/FLOOR`. With its optional checks it also gates
recursion, entry-point reachability (REACH), compile inputs, the C
caller's token mints (MINTS) and rlimit headroom (RLIMIT). It is
pcre3's `verus/count` made generic, with challoc's generated-ledger back
end added; `src/main.rs`'s module documentation states every rule.

## Running

```sh
cargo build --locked --release
verus-count [--config FILE] [--list] [--floor FILE [--base-rev REV]] [--rlimit STATS] ROOT
```

ROOT is the repository root (default `.`) and FILE the configuration
(default `ROOT/verus/count.toml`). `--list` prints one line per
executable function; `--floor` holds the `shipped` row to the FLOOR
file; `--base-rev` also holds FLOOR (and REACH and RLIMIT) to that
revision's copies, and fails when the revision does not resolve;
`--rlimit` reads the statistics `cargo verus ... -- --time-expanded
--output-json` printed. `--version` prints the counter's version. An
unknown option or a second ROOT fails the run (exit 2), so a misspelt
`--flor` cannot leave the floor unchecked.

## Configuration

`verus/count.toml`, every path relative to the root. Unknown keys are
an error. A check whose section is left out does not run.
[configs/pcre3.toml](configs/pcre3.toml) and
[configs/challoc.toml](configs/challoc.toml) are complete examples.

| section | key | meaning |
|---|---|---|
| `[crates]` | `dir` | each subdirectory with a `src/` is a crate, named for it |
| | `members` | `{ name, path, ships }` crates named explicitly; `ships` overrides the `publish = false` rule |
| | `test_cfgs` | `cfg` names only test builds set (default `test`, `kani`) |
| | `test_features` | features only tests enable; a workspace build that enables one fails the run (`cargo tree`) |
| | `verus_features` | features only Verus's build enables, as `verus_keep_ghost` |
| | `stand_ins` | `[from, to]` same-length replacements before parsing, for identifiers Verus keeps as keywords; one that would rewrite a name the counter classes code by (`external_body`, `assume`, `admit`, `verus_verify`, `discharged_by`, `cfg`, `fn`, a test cfg, a configured macro or mint, ...) is refused |
| | `verus_only_ghost` | count the ghost functions of Verus-only code in the ghost column |
| `[floor]` | `path`, `checked`, `renamed_from` | the FLOOR file (default `verus/FLOOR`); the prefix of the two verified-or-discharged figures (default `checked`); an older prefix the base revision's copy may still give them under, for the change that renames them (challoc's `combined`) |
| `[rlimit]` | `path` | the RLIMIT file (default `verus/RLIMIT`) |
| `[kani]` | `records` | the directory of `kani-*.md` records that cover the `discharged_by = "kani"` marker |
| `[seams]` | `tables`, `theorems` | the seams tables, and a directory whose every `*.md` holds one |
| `[recursion]` | `table` | turns the recursion check on; the table that names verified recursion |
| `[reach]` | `path`, `exports_macro`, `fn_ptr_macro`, `conversion` | turns REACH on; the port's exports macro, function-pointer macro and argument conversion, each optional |
| `[inputs]` | `table`, `crates`, `roots`, `lazy`, `describe` | turns the compile-inputs check on; a root that matches no function of `crates` fails it |
| `[mints]` | `path`, `mint`, `token_trait`, `home` | turns the token-mint gate on |
| `[ledger]` | `dir`, `isla`, `kani` | discharges from a generated seam ledger instead of the Kani marker; the premise files an Isla or Kani status may cite |

A minimal configuration is two lines:

```toml
[crates]
dir = "crates"
```

## Two ways to discharge a seam

- **The Kani marker** (pcre3): a trusted function carries
  `#[cfg_attr(any(), discharged_by = "kani")]` and a `kani-*.md` record
  quotes it. The report is one table with a `discharged` column, and the
  checked figures add the discharged functions.
- **The generated ledger** (challoc, `[ledger]`): `verus/seams.sh`
  writes `verus/ledger/**.rs.md` from the source, and each entry's
  status says whether its seam is discharged, partly checked or
  trusted. The report is challoc's two tables, the second the combined
  share. The marker is refused in this mode, so a function cannot be
  discharged twice.

Verus's attribute form (`#[verus_verify]`, `#[verus_spec(..)]`, alone
or under a `cfg_attr`) counts as code inside `verus!`, for a port whose
MSRV keeps vstd optional (the skill's proofs.md, "When vstd cannot be a
dependency of every build").

## Parity

Run on the same commits, the kit counter's full output (standard
output, standard error and `--list`) is byte-identical to each port's
in-tree counter, but for one line:

| port | commit | configuration | `--floor verus/FLOOR --base-rev HEAD^` | `--list` |
|---|---|---|---|---|
| pcre3 | wolfi-dev/pcre3 main `158cf87a` | `configs/pcre3.toml` | identical but the `ok compile inputs` line | identical but that line (3,919 lines) |
| challoc | wolfi-dev/challoc main `fcde6b2` | `configs/challoc.toml` | identical | identical (1,674 lines) |
| challoc | wolfi-dev/challoc main `39751a02` | `configs/challoc.toml` | identical (also with no flag, and `--list --floor verus/FLOOR --base-rev fcde6b2`) | identical (1,672 lines) |

The kit's `ok compile inputs` line also names each root it resolved
and where (`(roots Regex::new at crates/pcre3/src/regex.rs:274,
encode_as at crates/pcre3/src/serialize.rs:631)`); pcre3's does not
name them. pcre3's `--rlimit` path, with statistics written for the
purpose (every crate, every function with a `#[verifier::rlimit]`) and
`--base-rev HEAD^`, differs in the same line alone.

Where the counters still differ, on inputs neither port has today:

- pcre3's counter counts the commas of generic arguments in a call's
  arguments (`helper(make::<u8, u16>())`) as the call's, and drops the
  edge to a one-parameter `helper`; the kit leaves such a call's arity
  unknown. pcre3's counter with the same fix gives the same output on
  `158cf87a`.
- pcre3's counter lets a function that keeps its body but loses a
  `#[verifier::rlimit(N)]` below the default rise to the default with
  no `rlimit-raised` line; the kit requires one. pcre3 has no limit
  below the default. (challoc's counter has neither the call walk nor
  the rlimit check.)
- The kit refuses a stand-in that would rewrite a name it classes code
  by, and a compile-inputs root that matches no function of the
  configured crates; it credits Verus's attribute form under a
  `cfg_attr` only when Verus's build meets the predicate. Neither port
  has a stand-in, root or attribute these refuse.

- challoc's counter takes a missing `combined-*` figure in the FLOOR
  under test as 0.0; the kit's requires all four (a base revision's
  copy may still lack them). challoc's FLOOR has given all four since
  `4f7b840b`, so no base revision a pull request compares with lacks
  them.
- Renaming challoc's `combined-*` figures to `checked-*` (kit/PLAN.md,
  step 2) needs `renamed_from = "combined"` in the change that renames
  them: without it the base revision's copy names figures the
  configuration does not, and the run stops (`unknown figure
  "combined-functions"`, exit 2). With it, on `39751a02` with FLOOR's
  lines renamed, `--floor verus/FLOOR --base-rev HEAD` passes with the
  same four figures, and a `checked-lines` set 0.1 below the base's
  fails for want of a `lowered` line. The key may stay after the rename;
  it only widens what a base's copy may name.
- challoc's counter reads an inline `#[cfg(verus_keep_ghost)] mod { }`
  as shipped code; the kit reads it as Verus-only, as pcre3's does, so
  only its trusted functions count.
- challoc's ledger reads only `src/FILE:LINE` entries; the kit's reads
  any `FILE.rs:LINE`, so a ledger may cover several crates.
- The failure text for a ledger status that names a loader or kernel
  premise without its `callers:` clause, and for an unlisted compile
  input, words its reason from the configuration rather than from
  challoc's and pcre3's file names.

## Bootstrapping a port

A port with no counter and FLOOR 0.0 adopts the counter in its first
proof PR (the skill's lifecycle.md, phase 4):

1. Vendor the counter ("Adopting: vendoring", below), add
   `verus/count.toml` (the two-line form above is enough to start)
   and run the counter in the verus check:
   `verus-count --floor verus/FLOOR --base-rev "$BASE" .`.
2. Copy `kit/templates/verus/FLOOR`, whose figures are all 0.0.
3. On the PR's base the counter names 0.0 for every figure; on the PR it
   names the figures the proof reaches. Record those in FLOOR in the
   same PR, even where FLOOR's raise rule is "own PR": this first raise
   cannot come before the counter and the proof.

A port with a `verus/floor/` directory names the FLOOR file something
else (chainz: `verus/SHARES`), since `verus/FLOOR` and `verus/floor/`
are one path on a case-insensitive file system. Name it in both places:
the `--floor` flag, which the counter reads at the head, and
`[floor] path` in `verus/count.toml`, which it reads at `--base-rev`
(chainz: `--floor verus/SHARES` and `path = "verus/SHARES"`). With only
the key set, step 1's `--floor verus/FLOOR` reads the directory and the
counter exits 2.

On chainz's PR #6 head (`eb79512`), the two-line configuration counts
16 verified functions in zlib-core, the number chainz's own
`verus/FLOOR` records (`verified 16`, an older single-figure format
the kit does not read), and names `functions 4.1`, `lines 12.8`. On
expat-rs's U1 worktree, with `verus_features = ["verus"]`, it counts
`utf8::invalid_char` (57 lines) as verified, the only one, as the
pilot's hand count did.

## Adopting: vendoring

TODO: once proved-port (and the ports) are public, publish verus-count as a package (crates.io or a Wolfi apk installed as a pinned environment package) and replace vendoring.

A port's check guests cannot reach this repository, so a port carries
a copy of this crate at `verus/count/`, made by `kit/sync.sh` from a
clone of this repository:

```sh
kit/sync.sh PORT_ROOT          # copy kit/count at HEAD to PORT_ROOT/verus/count
kit/sync.sh --check PORT_ROOT  # fail unless the copy matches its recorded commit
```

- The copy is `git archive` of `kit/count` at the clone's HEAD, so it
  holds exactly what that commit holds: no `target/`, no uncommitted
  edit (the sync refuses while `kit/count` has any), and none of the
  files a later kit deletes. It replaces the whole directory.
- `verus/count/KIT` records the commit, and the tag when HEAD has one.
  The sync refuses a `verus/count/` with no `KIT` file: that is the
  port's own counter, for the port's PR to delete.
- `--check` fails when a file is added, deleted or edited or changes
  its executable bit, or when the recorded commit or tag does not
  resolve in the clone; it ignores `verus/count/target/`. The port's CI
  cannot reach this repository to run it, so whoever changes the copy
  runs it and names the commit in the PR.
- The crate builds where it lands: it is its own workspace (the empty
  `[workspace]` table) with its own `Cargo.lock`, and nothing in it
  names a path outside it. The port's verus check builds it with
  `cargo build --locked --release --manifest-path verus/count/Cargo.toml`
  (a `--target-dir` outside the copy keeps `git status` clean) and runs
  it as Bootstrapping, step 1, says.
- Moving to a newer kit is a port PR that re-runs the sync. Never edit
  the copy: a fix the port needs lands in `kit/count` first.

## Release

Ports will pin the kit as a released package (kit/PLAN.md, section 4)
once the TODO above is done; until then they vendor it. A
release of this crate needs:

- a version: `Cargo.toml` says `0.0.0` until the first `kit/vX.Y.Z` tag
  sets it, and `--version` prints it so a port's Makefile can refuse
  another;
- a package build (a melange config in a package repository the ports'
  check guests can read) that runs `cargo build --locked --release`
  here and installs `verus-count`, with `cargo test` in its test
  stage;
- the ports' checks to install it pinned with `~`, and to drop the
  in-tree counter and its `counter tests` step in the same PR.

No tag or release exists yet.
