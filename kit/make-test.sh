#!/bin/sh
# kit/make-test.sh: tests of kit/make (the make-template check). A
# scratch kit repository holds this tree's kit/make and kit/templates;
# kit/sync.sh vendors them into a scratch port whose Makefile includes
# verus/make/proved-port.mk. Then:
#   - `make -n` shows the commands each target runs, for the defaults
#     and for the settings pcre3, expat-rs, chainz and challoc need;
#   - kani.sh runs with a stub cargo: crate selection, one harness,
#     shards, jobs, a failure, the fail pattern, the version check, a
#     selection with no harnesses with and without KANI_NONE, one name
#     in several files;
#   - headers.sh holds the kit's Verus files (kit/make/VERUS-FILES) and
#     no other: it passes them as the port fills them and fails on
#     drift.
# No Verus or Kani runs. Run it from the repository root.
set -eu

export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# --- the scratch kit and port -------------------------------------------
kitrepo=$tmp/kit
mkdir -p "$kitrepo/kit/count"
cp kit/sync.sh "$kitrepo/kit/sync.sh"
cp -R kit/make kit/templates "$kitrepo/kit/"
echo '[package]' > "$kitrepo/kit/count/Cargo.toml"
git -C "$kitrepo" init -q
git -C "$kitrepo" add .
git -C "$kitrepo" -c user.name=make-test -c user.email=make-test@example.com \
  -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm kit
sha=$(git -C "$kitrepo" rev-parse HEAD)

port=$tmp/port
mkdir "$port"
sh "$kitrepo/kit/sync.sh" "$port" > /dev/null
[ -f "$port/verus/make/proved-port.mk" ] || fail "sync made no verus/make/proved-port.mk"
cat > "$port/Makefile" <<'EOF'
VERUS_CRATES = alpha beta
include verus/make/proved-port.mk
EOF

# shows TARGET [VAR=VALUE...]: `make -n TARGET` in the port, each run of
# spaces one space.
shows() {
  make -s -n -C "$port" "$@" | tr -s ' ' | sed 's/ $//'
}

# expect WHAT GOT WANT: GOT and WANT are the same lines.
expect() {
  if [ "$2" != "$3" ]; then
    printf 'got:\n%s\nwant:\n%s\n' "$2" "$3" >&2
    fail "$1"
  fi
  echo "ok $1"
}

count_build='cargo build --locked --release --manifest-path verus/count/Cargo.toml --target-dir target/verus-count'

# --- make -n ----------------------------------------------------------------
expect "make -n verus, the template's verus check" \
  "$(shows verus VERUS_THREADS=16 VERUS_BASE=abc123)" \
  "mkdir -p target/
cargo verus focus --locked -p alpha -p beta -- --time-expanded --output-json --num-threads 16 > target/verus-stats.json
$count_build
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --base-rev abc123 ."

# No base fails verus and verus-count before any command, with no SKIP:
# a check that loses its base must not pass.
for target in verus verus-count; do
  for base in '' 'VERUS_BASE='; do
    what="make -n $target${base:+ $base}"
    if make -s -n -C "$port" "$target" ${base:+"$base"} > "$tmp/nobase.out" 2> "$tmp/nobase.err"; then
      fail "$what: got = success, want = failure"
    fi
    grep -q 'no VERUS_BASE to compare verus/FLOOR with: give the revision' "$tmp/nobase.err" \
      || fail "$what: got = $(cat "$tmp/nobase.err"), want = the no-VERUS_BASE message"
    grep -q 'VERUS_BASE=none' "$tmp/nobase.err" \
      || fail "$what: the message does not name VERUS_BASE=none"
    [ ! -s "$tmp/nobase.out" ] \
      || fail "$what: got = $(cat "$tmp/nobase.out"), want = no command"
    echo "ok $what fails with no base"
  done
done
expect "make -n verus-proofs needs no base" \
  "$(shows verus-proofs)" \
  "mkdir -p target/
cargo verus focus --locked -p alpha -p beta -- --time-expanded --output-json > target/verus-stats.json"

expect "make -n verus VERUS_BASE=none, a local run (--no-base)" \
  "$(shows verus VERUS_BASE=none)" \
  "mkdir -p target/
cargo verus focus --locked -p alpha -p beta -- --time-expanded --output-json > target/verus-stats.json
$count_build
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."

expect "make -n verus, expat-rs (--features verus)" \
  "$(shows verus VERUS_CRATES=expat-rs VERUS_CARGO_FLAGS='--features verus' VERUS_THREADS=4 VERUS_BASE=HEAD^)" \
  "mkdir -p target/
cargo verus focus --locked -p expat-rs --features verus -- --time-expanded --output-json --num-threads 4 > target/verus-stats.json
$count_build
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --base-rev HEAD^ ."

expect "make -n verus, chainz (verus/SHARES)" \
  "$(shows verus VERUS_CRATES=zlib-core VERUS_FLOOR=verus/SHARES VERUS_BASE=main)" \
  "mkdir -p target/
cargo verus focus --locked -p zlib-core -- --time-expanded --output-json > target/verus-stats.json
$count_build
target/verus-count/release/verus-count --floor verus/SHARES --rlimit target/verus-stats.json --base-rev main ."

expect "make -n verus, challoc (verify, no RLIMIT yet)" \
  "$(shows verus VERUS_MODE=verify VERUS_CRATES= VERUS_RLIMIT= VERUS_BASE=main)" \
  "mkdir -p target/
cargo verus verify --locked -- --time-expanded --output-json > target/verus-stats.json
$count_build
target/verus-count/release/verus-count --floor verus/FLOOR --base-rev main ."

expect "make -n verus-proofs, challoc's second quantum" \
  "$(shows verus-proofs VERUS_MODE=verify VERUS_CRATES= VERUS_CARGO_FLAGS='--no-default-features --features quantum8' VERUS_STATS=target/q8/stats.json)" \
  "mkdir -p target/q8/
cargo verus verify --locked --no-default-features --features quantum8 -- --time-expanded --output-json > target/q8/stats.json"

expect "make -n verus-count-test" \
  "$(shows verus-count-test)" \
  "cargo test --locked --manifest-path verus/count/Cargo.toml --target-dir target/verus-count"

expect "make -n verus-headers" "$(shows verus-headers)" "VERUS_FLOOR=verus/FLOOR sh verus/make/headers.sh ."
expect "make -n verus-headers, chainz (verus/SHARES)" "$(shows verus-headers VERUS_FLOOR=verus/SHARES)" \
  "VERUS_FLOOR=verus/SHARES sh verus/make/headers.sh ."
expect "make -n kani" "$(shows kani)" "sh verus/make/kani.sh"
expect "make -n kani-list" "$(shows kani-list)" "sh verus/make/kani.sh --list"

# A prerequisite the port adds runs before the recipe.
cat > "$port/Makefile" <<'EOF'
VERUS_CRATES = alpha
include verus/make/proved-port.mk
verus: verus-scan
verus-scan:
	sh verus/scan.sh
EOF
shows verus VERUS_BASE=x | head -n 1 > "$tmp/first"
expect "a port's prerequisite runs first" "$(cat "$tmp/first")" "sh verus/scan.sh"

# --- kani.sh with a stub cargo ----------------------------------------------
bin=$tmp/bin
mkdir -p "$bin" "$tmp/lists"
log=$tmp/cargo.log
# The stub logs each call, answers --version, writes kani-list.json from
# $tmp/lists/<crate>.json for `list` (<crate>-quantum8.json when the call
# has `--features quantum8` and that file exists, as a feature-gated
# harness is listed only with its feature), fails a run of crate
# $STUB_FAIL and prints $STUB_SAY in each run. As Kani 0.68.0's does, a
# separate `-p` takes every bare word after it, so `-p CRATE list` is a
# usage error (exit 2).
cat > "$bin/cargo" <<EOF
#!/bin/sh
echo "\$*" >> "$log"
[ "\$1" = fetch ] && exit 0
[ "\$2" = --version ] && { echo "cargo-kani \${STUB_VERSION:-0.68.0}"; exit 0; }
crate=default
if [ "\$2" = -p ]; then
  crate=\$3
  [ "\${4-}" != list ] || { echo "error: unexpected argument '--format' found" >&2; exit 2; }
fi
case "\$2" in -p=*) crate=\${2#-p=} ;; esac
json=$tmp/lists/\$crate.json
case " \$* " in
  *" --features quantum8 "*) [ ! -f "$tmp/lists/\$crate-quantum8.json" ] || json=$tmp/lists/\$crate-quantum8.json ;;
esac
case " \$* " in
  *" list "*) cp "\$json" kani-list.json; exit 0 ;;
esac
echo "Checking harness of \$crate"
[ -z "\${STUB_SAY:-}" ] || echo "\$STUB_SAY"
[ "\$crate" != "\${STUB_FAIL:-}" ]
EOF
chmod +x "$bin/cargo"

# list CRATE TOTAL NAME...: CRATE's kani-list.json, with TOTAL in its
# totals.
list() {
  c=$1 total=$2
  shift 2
  {
    echo '{'
    echo '  "kani-version": "0.68.0",'
    echo '  "file-version": "0.1",'
    echo '  "standard-harnesses": {'
    echo '    "src/lib.rs": ['
    for h in "$@"; do echo "      \"$h\","; done
    echo '    ]'
    echo '  },'
    echo '  "contract-harnesses": {},'
    echo '  "contracts": [],'
    echo '  "totals": {'
    echo "    \"standard-harnesses\": $total,"
    echo '    "contract-harnesses": 0,'
    echo '    "functions-under-contract": 0'
    echo '  }'
    echo '}'
  } > "$tmp/lists/$c.json"
}
list alpha 3 kani::a_one kani::a_two kani::a_three
list beta 1 kani::b_one
list default 1 kani::d_one

# kani [VAR=VALUE...] -- [ARGS]: kani.sh in the port with the stub cargo,
# the calls it made (all but fetch) in $tmp/calls, its output in
# $tmp/out; its exit status.
kani() {
  : > "$log"
  rc=0
  (cd "$port" && env PATH="$bin:$PATH" "$@" > "$tmp/out" 2>&1) || rc=$?
  grep -v '^fetch' "$log" > "$tmp/calls" || true
  return $rc
}

# runs WHAT WANT VAR=VALUE... [sh verus/make/kani.sh ARGS]: it passes,
# and its cargo kani calls are WANT.
runs() {
  what=$1 want=$2
  shift 2
  kani "$@" || { cat "$tmp/out" >&2; fail "$what: kani.sh failed"; }
  grep -v -x 'kani --version' "$log" | head -n 1 | grep -qx 'fetch --locked' ||
    fail "$what: no cargo fetch --locked before the kani calls"
  expect "kani.sh $what" "$(cat "$tmp/calls")" "$want"
}

# refuses WHAT CODE VAR=VALUE...: it exits CODE.
refuses() {
  what=$1 code=$2
  shift 2
  rc=0
  kani "$@" || rc=$?
  [ "$rc" -eq "$code" ] || { cat "$tmp/out" >&2; fail "kani.sh $what: exit $rc, want $code"; }
  echo "ok kani.sh $what exits $code"
}

k=verus/make/kani.sh
# A run of every harness lists them first (cargo kani passes a crate
# with none), then runs each crate.
la='kani -p=alpha list --format json'
lb='kani -p=beta list --format json'
runs "runs each crate" "$la
$lb
kani -p=alpha
kani -p=beta" KANI_CRATES="alpha beta" sh "$k"
runs "runs cargo's default selection with no KANI_CRATES" "kani list --format json
kani" sh "$k"
runs "KANI_ONLY" "$la
kani -p=alpha" KANI_CRATES="alpha beta" KANI_ONLY=alpha sh "$k"
runs "KANI_EXCLUDE" "$lb
kani -p=beta" KANI_CRATES="alpha beta" KANI_EXCLUDE=alpha sh "$k"
runs "KANI_HARNESS (chainz's foreach), with no listing" "kani -p=beta --harness b_one" \
  KANI_CRATES="alpha beta" KANI_ONLY=beta KANI_HARNESS=b_one sh "$k"
runs "KANI_JOBS (pcre3)" "$la
kani -p=alpha -j 4 --output-format terse" \
  KANI_CRATES=alpha KANI_JOBS=4 sh "$k"
runs "flags, ARGS and CBMC arguments (challoc)" \
  "kani -Z stubbing --features quantum16 -Z extra list --format json
kani -Z stubbing --features quantum16 --output-format regular -Z extra --cbmc-args --verbosity 9" \
  KANI_FLAGS="-Z stubbing --features quantum16" KANI_FORMAT=regular \
  KANI_CBMC_ARGS="--verbosity 9" sh "$k" -Z extra
runs "KANI_VERSION it has" "kani --version
$la
kani -p=alpha" KANI_CRATES=alpha KANI_VERSION=0.68.0 sh "$k"

# No harnesses: a selection with none fails, since cargo kani would pass
# it having checked nothing, unless KANI_NONE=yes declares it; under the
# declaration a harness fails the run, so it cannot hide one.
list gamma 0
lg='kani -p=gamma list --format json'
refuses "a selection with no harnesses" 1 KANI_CRATES=gamma sh "$k"
expect "kani.sh runs nothing for a selection with no harnesses" "$(cat "$tmp/calls")" "$lg"
grep -q 'have no harnesses, so nothing would run; a port with none yet sets KANI_NONE = yes' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "kani.sh with no harnesses: got = no message naming KANI_NONE, want = one"; }
refuses "KANI_ONLY on the crate with no harnesses" 1 KANI_CRATES="alpha gamma" KANI_ONLY=gamma sh "$k"
# cargo's default selection whose last harness is gone: an ordinary,
# unsharded run, which cargo kani would pass.
list default 0
refuses "cargo's default selection with no harnesses" 1 sh "$k"
expect "kani.sh runs nothing for a default selection with no harnesses" "$(cat "$tmp/calls")" "kani list --format json"
list default 1 kani::d_one
runs "a crate with no harnesses beside one with some" "$la
$lg
kani -p=alpha
kani -p=gamma" KANI_CRATES="alpha gamma" sh "$k"
runs "KANI_NONE=yes with no harnesses" "$lg" KANI_CRATES=gamma KANI_NONE=yes sh "$k"
grep -qx 'kani: no harnesses, as KANI_NONE=yes declares' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "kani.sh KANI_NONE=yes: got = no declaration line, want = it"; }
refuses "KANI_NONE=yes with harnesses" 1 KANI_CRATES="alpha gamma" KANI_NONE=yes sh "$k"
expect "kani.sh runs no harness under a stale KANI_NONE" "$(cat "$tmp/calls")" "$la
$lg"
grep -q 'KANI_NONE=yes declares no harnesses, but the selected crates have 3; delete KANI_NONE' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "kani.sh stale KANI_NONE: got = no message, want = one naming the 3 harnesses"; }
refuses "KANI_NONE=yes over cargo's default selection, which has a harness" 1 KANI_NONE=yes sh "$k"
refuses "a KANI_NONE other than yes" 2 KANI_CRATES=gamma KANI_NONE=1 sh "$k"
refuses "KANI_NONE=yes with KANI_HARNESS" 2 KANI_CRATES=gamma KANI_NONE=yes KANI_HARNESS=g sh "$k"
refuses "KANI_NONE=yes with KANI_SHARD" 2 KANI_CRATES=gamma KANI_NONE=yes KANI_SHARD=1 KANI_SHARDS=1 sh "$k"
refuses "a shard of a selection with no harnesses" 1 KANI_CRATES=gamma KANI_SHARD=1 KANI_SHARDS=1 sh "$k"

# A list file in another shape than Kani 0.68.0's would read as no
# harnesses, which KANI_NONE=yes passes, so it fails: written on one
# line, with no totals, or with a total missing.
printf '{"kani-version":"0.68.0","standard-harnesses":{"src/lib.rs":["kani::o_one"]},"totals":{"standard-harnesses":1,"contract-harnesses":0}}\n' \
  > "$tmp/lists/odd.json"
refuses "KANI_NONE=yes over a list file on one line" 1 KANI_CRATES=odd KANI_NONE=yes sh "$k"
grep -q 'has no "totals" block giving "standard-harnesses" and "contract-harnesses"' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "kani.sh with a one-line list file: got = no message naming the totals, want = one"; }
list odd 1 kani::o_one
grep -v -e totals -e '-harnesses": ' -e 'functions-under-contract' "$tmp/lists/odd.json" > "$tmp/odd" &&
  cp "$tmp/odd" "$tmp/lists/odd.json"
refuses "KANI_NONE=yes over a list file with no totals" 1 KANI_CRATES=odd KANI_NONE=yes sh "$k"
list odd 0
grep -v '"contract-harnesses": ' "$tmp/lists/odd.json" > "$tmp/odd" && cp "$tmp/odd" "$tmp/lists/odd.json"
refuses "KANI_NONE=yes over a list file with no contract-harnesses total" 1 KANI_CRATES=odd KANI_NONE=yes sh "$k"

refuses "on a failing crate" 1 KANI_CRATES="alpha beta" STUB_FAIL=alpha sh "$k"
grep -qx 'kani -p=beta' "$tmp/calls" || fail "kani.sh stopped at the failing crate"
echo "ok kani.sh runs the other crates after a failure"
refuses "KANI_ONLY and KANI_EXCLUDE on one crate" 1 KANI_CRATES="alpha beta" KANI_ONLY=alpha KANI_EXCLUDE=alpha sh "$k"
refuses "a KANI_ONLY not in KANI_CRATES" 2 KANI_CRATES="alpha beta" KANI_ONLY=gamma sh "$k"
refuses "KANI_ONLY with no KANI_CRATES" 2 KANI_ONLY=alpha sh "$k"
refuses "KANI_HARNESS over two crates" 2 KANI_CRATES="alpha beta" KANI_HARNESS=a_one sh "$k"
refuses "KANI_JOBS with regular output" 2 KANI_JOBS=4 KANI_FORMAT=regular sh "$k"
refuses "KANI_SHARD past KANI_SHARDS" 2 KANI_SHARD=3 KANI_SHARDS=2 sh "$k"
refuses "KANI_SHARDS 0" 2 KANI_SHARD=0 KANI_SHARDS=0 sh "$k"
refuses "a KANI_VERSION it lacks" 1 KANI_VERSION=0.67.0 sh "$k"
refuses "the fail pattern" 1 KANI_CRATES=alpha STUB_SAY='warning: ignoring >=' KANI_FAIL_PATTERN='warning: ignoring' sh "$k"
grep -q 'warning: ignoring' "$tmp/out" || fail "kani.sh did not print the line with the fail pattern"
runs "the fail pattern, absent" "$la
kani -p=alpha" KANI_CRATES=alpha STUB_SAY='VERIFICATION:- SUCCESSFUL' KANI_FAIL_PATTERN='warning: ignoring' sh "$k"
grep -qx 'VERIFICATION:- SUCCESSFUL' "$tmp/out" || fail "kani.sh with a fail pattern did not print the result lines"

kani KANI_CRATES="alpha beta" sh "$k" --list || fail "kani.sh --list failed"
expect "kani.sh --list" "$(grep -v '^kani:' "$tmp/out")" "alpha kani::a_one src/lib.rs
alpha kani::a_three src/lib.rs
alpha kani::a_two src/lib.rs
beta kani::b_one src/lib.rs"
grep -q ' list ' "$tmp/calls" || fail "kani.sh --list did not call cargo kani list"
if grep -v ' list ' "$tmp/calls" | grep -q .; then fail "kani.sh --list ran a harness"; fi
# Each place the vendored kani.sh and proved-port.mk (the kani-list
# target's header row and its help) say what --list prints names the
# columns it prints.
grep -v '^kani:' "$tmp/out" | awk '{ print NF }' | LC_ALL=C sort -u > "$tmp/cols"
grep -h -o -E 'one `?CRATE NAME[A-Z ]*' "$port/verus/make/kani.sh" "$port/verus/make/proved-port.mk" |
  awk '{ print NF - 1 }' > "$tmp/doc-cols"
expect "kani.sh, the kani-list target and its help each document --list" "$(wc -l < "$tmp/doc-cols" | tr -d ' ')" 3
expect "the docs of --list name the $(cat "$tmp/cols") columns it prints" "$(LC_ALL=C sort -u "$tmp/doc-cols")" "$(cat "$tmp/cols")"

list beta 2 kani::b_one
refuses "a kani-list.json whose totals do not match" 1 KANI_CRATES=beta KANI_SHARD=1 KANI_SHARDS=1 sh "$k"
list beta 1 kani::b_one

# Shards: a_one marked alone takes shard 1; the rest, longest first, go to
# the least-loaded of shards 2 and 3.
printf '# NAME SECONDS\nkani::a_one 100 alone\nkani::a_two 50\nkani::b_one 40\n' > "$port/weights"
shard() {
  runs "$1" "$2" KANI_CRATES="alpha beta" KANI_WEIGHTS=weights KANI_SHARDS=3 KANI_SHARD="$3" sh "$k"
}
lists="kani -p=alpha list --format json
kani -p=beta list --format json"
shard "shard 1 of 3: the harness marked alone" "$lists
kani -p=alpha --exact --harness kani::a_one" 1
shard "shard 2 of 3" "$lists
kani -p=alpha --exact --harness kani::a_two" 2
shard "shard 3 of 3: one call a crate" "$lists
kani -p=alpha --exact --harness kani::a_three
kani -p=beta --exact --harness kani::b_one" 3
shard "shard 0 runs nothing" "$lists" 0
grep -q 'shard 1: 1 harnesses, ~100 s' "$tmp/out" || fail "shard 0 did not print the split"

# Harnesses marked alone that fill every shard leave the rest to no
# shard, so every shard refuses rather than pass without running them.
printf 'kani::a_one 100 alone\nkani::a_two 50 alone\n' > "$port/weights2"
for s in 0 1 2; do
  refuses "shard $s of 2 with both shards marked alone" 2 \
    KANI_CRATES="alpha beta" KANI_WEIGHTS=weights2 KANI_SHARDS=2 KANI_SHARD="$s" sh "$k"
  grep -q 'fill all 2 shards, leaving 2 harnesses no shard' "$tmp/out" ||
    { cat "$tmp/out" >&2; fail "shard $s of 2: got = no message naming the harnesses left over, want = one"; }
  if grep -q -- '--exact' "$tmp/calls"; then fail "shard $s of 2 ran harnesses before refusing"; fi
done
runs "two shards marked alone of three leave shard 3 for the rest" "$lists
kani -p=alpha --exact --harness kani::a_three
kani -p=beta --exact --harness kani::b_one" \
  KANI_CRATES="alpha beta" KANI_WEIGHTS=weights2 KANI_SHARDS=3 KANI_SHARD=3 sh "$k"

# A harness is its name and its file. cargo's default selection over a
# workspace whose crates a, b and c each have a tests::check (c's a
# contract harness) has four harnesses: the kani-list.json Kani 0.68.0
# wrote for it, whose "contracts" array names c's a second time.
{
  echo '{'
  echo '  "kani-version": "0.68.0",'
  echo '  "file-version": "0.1",'
  echo '  "standard-harnesses": {'
  echo '    "a/src/lib.rs": ['
  echo '      "tests::check"'
  echo '    ],'
  echo '    "b/src/lib.rs": ['
  echo '      "tests::check",'
  echo '      "tests::other"'
  echo '    ]'
  echo '  },'
  echo '  "contract-harnesses": {'
  echo '    "c/src/lib.rs": ['
  echo '      "tests::check"'
  echo '    ]'
  echo '  },'
  echo '  "contracts": ['
  echo '    {'
  echo '      "function": "half",'
  echo '      "file": "c/src/lib.rs",'
  echo '      "harnesses": ['
  echo '        "tests::check"'
  echo '      ]'
  echo '    }'
  echo '  ],'
  echo '  "totals": {'
  echo '    "standard-harnesses": 3,'
  echo '    "contract-harnesses": 1,'
  echo '    "functions-under-contract": 1'
  echo '  }'
  echo '}'
} > "$tmp/lists/default.json"
runs "cargo's default selection with tests::check in three files" "kani list --format json
kani" sh "$k"
kani sh "$k" --list || { cat "$tmp/out" >&2; fail "kani.sh --list over tests::check in three files failed"; }
expect "kani.sh --list keeps each file's tests::check" "$(grep -v '^kani:' "$tmp/out")" "- tests::check a/src/lib.rs
- tests::check b/src/lib.rs
- tests::check c/src/lib.rs
- tests::other b/src/lib.rs"
refuses "KANI_NONE=yes over tests::check in three files" 1 KANI_NONE=yes sh "$k"
grep -q 'the selected crates have 4; delete KANI_NONE' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "kani.sh KANI_NONE=yes over tests::check in three files: got = no message naming 4 harnesses, want = one"; }
# Sharded, --exact --harness tests::check runs all three, so they share
# a shard: 150 s on shard 1, tests::other on shard 2. Split by file, two
# would go to shard 1 and one to shard 2, which runs all three again.
printf 'tests::check 50\ntests::other 40\n' > "$port/weights3"
runs "shard 1 of 2 runs tests::check of every file" "kani list --format json
kani --exact --harness tests::check" KANI_WEIGHTS=weights3 KANI_SHARDS=2 KANI_SHARD=1 sh "$k"
grep -qx 'kani: shard 1 of 2: 3 of 4 harnesses' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "shard 1 of 2 over tests::check in three files: got = no count of 3 of 4, want = it"; }
grep -q 'shard 1: 3 harnesses, ~150 s' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "shard 1 of 2 over tests::check in three files: got = no split line of 3 harnesses, ~150 s, want = it"; }
runs "shard 2 of 2 runs tests::other alone" "kani list --format json
kani --exact --harness tests::other" KANI_WEIGHTS=weights3 KANI_SHARDS=2 KANI_SHARD=2 sh "$k"
kani KANI_SHARDS=2 KANI_SHARD=1 sh "$k" --list || fail "kani.sh --list of shard 1 of 2 failed"
expect "kani.sh --list of a shard keeps each file's tests::check" "$(grep -v '^kani:' "$tmp/out")" "- tests::check a/src/lib.rs
- tests::check b/src/lib.rs
- tests::check c/src/lib.rs"
# A harness map line in another shape fails rather than be skipped: a
# file key with a space.
{
  echo '{'
  echo '  "standard-harnesses": {'
  echo '    "a b/src/lib.rs": ['
  echo '      "tests::check"'
  echo '    ]'
  echo '  },'
  echo '  "contract-harnesses": {},'
  echo '  "totals": {'
  echo '    "standard-harnesses": 1,'
  echo '    "contract-harnesses": 0'
  echo '  }'
  echo '}'
} > "$tmp/lists/default.json"
refuses "a harness map line in another shape" 1 sh "$k"
grep -q 'not a line of a harness map as Kani 0.68.0 writes it' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "kani.sh with a file key holding a space: got = no message naming the line, want = one"; }
list default 1 kani::d_one

# ARGS reach the harness listing too: a harness gated on quantum8 is
# listed, so its shard runs it.
list alpha-quantum8 4 kani::a_one kani::a_two kani::a_three kani::a_q8
runs "ARGS reach cargo kani list (challoc's quantum8)" \
  "kani -p=alpha --no-default-features --features quantum8 list --format json
kani -p=alpha --exact --harness kani::a_one --harness kani::a_q8 --harness kani::a_three --harness kani::a_two --no-default-features --features quantum8" \
  KANI_CRATES=alpha KANI_SHARDS=1 KANI_SHARD=1 sh "$k" --no-default-features --features quantum8
kani KANI_CRATES=alpha sh "$k" --list --no-default-features --features quantum8 || fail "kani.sh --list ARGS failed"
expect "kani.sh --list ARGS lists the feature-gated harness" "$(grep -v '^kani:' "$tmp/out")" "alpha kani::a_one src/lib.rs
alpha kani::a_q8 src/lib.rs
alpha kani::a_three src/lib.rs
alpha kani::a_two src/lib.rs"
runs "list flags (challoc)" "kani -Z stubbing list -Z mem-predicates --format json
kani -Z stubbing --exact --harness kani::d_one" \
  KANI_FLAGS="-Z stubbing" KANI_LIST_FLAGS="-Z mem-predicates" KANI_SHARDS=1 KANI_SHARD=1 sh "$k"

# Through make: the snippet exports the KANI_ variables to kani.sh.
: > "$log"
(cd "$port" && PATH="$bin:$PATH" make -s kani KANI_CRATES=alpha KANI_JOBS=2 > "$tmp/out" 2>&1) ||
  { cat "$tmp/out" >&2; fail "make kani failed"; }
expect "make kani passes the KANI_ variables" "$(grep -v '^fetch' "$log")" "$la
kani -p=alpha -j 2 --output-format terse"

# A port with no harnesses yet declares it in its Makefile: make kani
# passes while it has none and fails once it has one.
cp "$port/Makefile" "$tmp/Makefile"
cat > "$port/Makefile" <<'EOF'
KANI_NONE = yes
include verus/make/proved-port.mk
EOF
(cd "$port" && PATH="$bin:$PATH" make -s kani KANI_CRATES=gamma > "$tmp/out" 2>&1) ||
  { cat "$tmp/out" >&2; fail "make kani with the Makefile's KANI_NONE = yes and no harnesses: got = failure, want = success"; }
grep -qx 'kani: no harnesses, as KANI_NONE=yes declares' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "make kani with KANI_NONE = yes: got = no declaration line, want = it"; }
echo "ok make kani passes a port whose Makefile declares KANI_NONE = yes and has no harnesses"
if (cd "$port" && PATH="$bin:$PATH" make -s kani KANI_CRATES=alpha > "$tmp/out" 2>&1); then
  cat "$tmp/out" >&2
  fail "make kani with the Makefile's KANI_NONE = yes and harnesses: got = success, want = failure"
fi
echo "ok make kani fails a port whose Makefile declares KANI_NONE = yes and has harnesses"
cp "$tmp/Makefile" "$port/Makefile"
if (cd "$port" && PATH="$bin:$PATH" make -s kani KANI_CRATES=gamma > "$tmp/out" 2>&1); then
  cat "$tmp/out" >&2
  fail "make kani with no harnesses and no KANI_NONE: got = success, want = failure"
fi
echo "ok make kani fails a port with no harnesses that does not declare KANI_NONE"
(cd "$port" && PATH="$bin:$PATH" make -s kani-list KANI_CRATES=alpha KANI_ARGS='--no-default-features --features quantum8' > "$tmp/out" 2>&1) ||
  { cat "$tmp/out" >&2; fail "make kani-list KANI_ARGS failed"; }
grep -qx 'alpha kani::a_q8 src/lib.rs' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "make kani-list KANI_ARGS: got = no kani::a_q8, want = the harness quantum8 gates"; }
echo "ok make kani-list passes KANI_ARGS to the listing"

# --- headers.sh ----------------------------------------------------------
templates=$port/verus/make/templates
# tpl FILE: the vendored template of the port's FILE, FILE.tmpl.
tpl() {
  echo "$templates/$1.tmpl"
}
# fill FILE: the port's copy of template FILE, its kit line in the
# templates' `vX.Y.Z (SHA)` form naming the scratch kit's commit, which
# has no tag yet, so the version is the release before it.
fill() {
  mkdir -p "$port/$(dirname "$1")"
  sed "s|^# kit: proved-port .*|# kit: proved-port v0.1.0 ($sha)|" "$(tpl "$1")" > "$port/$1"
}
headers() {
  rc=0
  (cd "$port" && sh verus/make/headers.sh) > "$tmp/out" 2>&1 || rc=$?
  return $rc
}
(cd "$templates" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) > "$tmp/vendored"
# Every vendored template ends in .tmpl, so that none ends in the name
# of a file a port's code owners protect (reviewers.yaml).
if grep -v '\.tmpl$' "$tmp/vendored"; then
  fail "sync vendored the templates above without .tmpl"
fi
echo "ok sync vendors each template as NAME.tmpl"
sed 's|\.tmpl$||' "$tmp/vendored" > "$tmp/templates"
# The vendored templates are the kit's Verus files, kit/make/VERUS-FILES,
# and no other.
grep -v -e '^#' -e '^$' kit/make/VERUS-FILES | LC_ALL=C sort > "$tmp/verus-files"
expect "sync vendors the kit's Verus files, kit/make/VERUS-FILES" "$(cat "$tmp/templates")" "$(cat "$tmp/verus-files")"
expect "kit/make/VERUS-FILES names the Verus files" "$(cat "$tmp/verus-files")" ".chainguard/ci/miri.yaml
.chainguard/ci/verus.yaml
.chainguard/review-prompt.md
.chainguard/reviewers.yaml
verus/FLOOR
verus/MINTS
verus/REACH
verus/RLIMIT
verus/count.toml"
while IFS= read -r f; do fill "$f"; done < "$tmp/templates"
# The port's other check files are its own shape, which headers.sh does
# not hold: each here has no kit header at all.
for f in .chainguard/ci/conformance.yaml .chainguard/ci/consumers.yaml .chainguard/ci/fuzz.yaml \
  .chainguard/ci/test.yaml .chainguard/ci/lint.yaml .chainguard/ci/env/default.yaml \
  .chainguard/ci/env/dev.yaml .chainguard/develop.yaml .chainguard/deps.yaml; do
  [ -f "kit/templates/$f" ] || fail "kit/templates has no $f to stand in for"
  mkdir -p "$port/$(dirname "$f")"
  printf '# The port'"'"'s own %s, not the kit'"'"'s.\nversion: 1\n' "$f" > "$port/$f"
done
headers || { cat "$tmp/out" >&2; fail "headers.sh failed on the templates as they are"; }
expect "headers.sh checks each Verus file and nothing else" "$(LC_ALL=C sort "$tmp/out")" "$(sed 's/^/ok /' "$tmp/verus-files")"
echo "ok headers.sh passes the templates as they are, kit lines in the vX.Y.Z (SHA) form, beside non-Verus check files of the port's own shape"

# The review prompt: the vendored-kit rule, rewrapped among the port's
# own text, passes; reworded or gone, it fails.
awk '
  /^- The vendored kit/ { print "- A check of the port'"'"'s own."; print "- The vendored kit"; sub(/^- The vendored kit/, ""); print; next }
  { print }
  /^## / { print "A paragraph of the port'"'"'s own." }' "$(tpl .chainguard/review-prompt.md)" > "$port/.chainguard/review-prompt.md"
headers || { cat "$tmp/out" >&2; fail "headers.sh failed on a rewrapped vendored-kit rule"; }
grep -qx 'ok .chainguard/review-prompt.md' "$tmp/out" || fail "headers.sh did not check .chainguard/review-prompt.md"
echo "ok headers.sh passes the vendored-kit rule rewrapped among the port's own text"
cp "$(tpl .chainguard/review-prompt.md)" "$port/.chainguard/review-prompt.md"

# A port fills its placeholders, rewrapping the paragraph, deletes the
# ones that are whole list items, and adds lines after its kit line.
awk '
  /[{]RAISE_RULE/ { print "# own PR: the figures rise in their own pull requests, to the figures"; print "# the check names."; skip = 1 }
  skip { if (/}/) skip = 0; next }
  /[{]SHIPPED_CRATES/ { sub(/[{]SHIPPED_CRATES.*/, "every crate of the port") ; print; skip2 = 1; next }
  skip2 { if (/}/) { sub(/.*}/, "#"); skip2 = 0 } else next }
  { print }
  /^# kit:/ { print "# A comment of the port'"'"'s own." }' "$port/verus/FLOOR" > "$tmp/FLOOR"
cp "$tmp/FLOOR" "$port/verus/FLOOR"
awk '/^# - [{]PORT_SPECIFIC_ROOTS/ { skip = 1 } skip { if (/}/) skip = 0; next } { print }' \
  "$port/verus/REACH" > "$tmp/REACH"
cp "$tmp/REACH" "$port/verus/REACH"
for f in verus/FLOOR verus/REACH; do
  [ "$(wc -l < "$port/$f")" -lt "$(wc -l < "$(tpl "$f")")" ] ||
    fail "the fixture deleted no placeholder lines from $f"
done
sed "s|^# kit: proved-port .*|# kit: proved-port $(echo "$sha" | cut -c1-12)|" "$port/verus/RLIMIT" > "$tmp/RLIMIT"
cp "$tmp/RLIMIT" "$port/verus/RLIMIT"
rm "$port/verus/MINTS"
headers || { cat "$tmp/out" >&2; fail "headers.sh failed on filled placeholders"; }
grep -qx 'skip verus/MINTS: the port has none' "$tmp/out" || fail "headers.sh did not name the file it skipped"
echo "ok headers.sh passes filled, rewrapped and deleted placeholders, a one-word short sha, a missing file"

# bad FILE WHAT AWK: FILE edited by the awk program AWK fails.
bad() {
  cp "$port/$1" "$tmp/keep"
  awk "$3" "$tmp/keep" > "$port/$1"
  if headers; then
    cat "$tmp/out" >&2
    fail "headers.sh passed $2"
  fi
  grep -q "headers: $1:" "$tmp/out" || { cat "$tmp/out" >&2; fail "headers.sh did not name $1 ($2)"; }
  cp "$tmp/keep" "$port/$1"
  echo "ok headers.sh fails $2"
}
bad verus/FLOOR "a header line reworded" 'NR == 2 { print "# something the kit never said"; next } { print }'
bad .chainguard/ci/verus.yaml "a header line deleted" 'NR == 3 { next } { print }'
bad verus/count.toml "a count.toml header line reworded" 'NR == 2 { print "# something the kit never said"; next } { print }'
bad .chainguard/ci/miri.yaml "a miri.yaml header line deleted" 'NR == 2 { next } { print }'
bad .chainguard/reviewers.yaml "a reviewers.yaml header line reworded" 'NR == 2 { print "# something the kit never said"; next } { print }'
bad .chainguard/review-prompt.md "the vendored-kit rule reworded" '/^- The vendored kit/ { on = 1 } on && /KIT/ { sub(/KIT/, "KITS"); on = 0 } { print }'
bad .chainguard/review-prompt.md "the vendored-kit rule gone" '/^- The vendored kit/ { skip = 1; next } skip && (/^- / || /^$/) { skip = 0 } skip { next } { print }'

# badtemplate FILE WHAT AWK MESSAGE: the vendored template of FILE edited
# by AWK so that it would hold nothing fails every port, with MESSAGE,
# rather than pass the port's copy unchecked.
badtemplate() {
  cp "$(tpl "$1")" "$tmp/keep"
  awk "$3" "$tmp/keep" > "$(tpl "$1")"
  rc=0
  headers || rc=$?
  [ "$rc" -eq 1 ] || { cat "$tmp/out" >&2; fail "headers.sh: $2: got = exit $rc, want = exit 1"; }
  grep -qF "$4" "$tmp/out" || { cat "$tmp/out" >&2; fail "headers.sh: $2: got = no message naming it, want = $4"; }
  cp "$tmp/keep" "$(tpl "$1")"
  echo "ok headers.sh fails $2"
}
badtemplate .chainguard/review-prompt.md "a template whose vendored-kit item is reworded" \
  '{ sub(/^- The vendored kit/, "- The kit copies") } { print }' \
  "verus/make/templates/.chainguard/review-prompt.md.tmpl has no \`- The vendored kit\` item"
badtemplate .chainguard/review-prompt.md "a template whose vendored-kit item is gone" \
  '/^- The vendored kit/ { skip = 1; next } skip && (/^- / || /^$/) { skip = 0 } skip { next } { print }' \
  "verus/make/templates/.chainguard/review-prompt.md.tmpl has no \`- The vendored kit\` item"
badtemplate verus/count.toml "a template with no kit line" \
  '/^# kit:/ { next } { print }' \
  "verus/make/templates/verus/count.toml.tmpl has no \`# kit: proved-port\` line, so it holds nothing"
bad verus/RLIMIT "a kit line naming another commit" '/^# kit:/ { print "# kit: proved-port 0123456789abcdef"; next } { print }'
bad verus/RLIMIT "a kit line naming a tag the kit has not" '/^# kit:/ { print "# kit: proved-port kit/v0.1.0"; next } { print }'
bad verus/RLIMIT "a kit line with too short a sha" "/^# kit:/ { print \"# kit: proved-port $(echo "$sha" | cut -c1-6)\"; next } { print }"
bad verus/RLIMIT "a vX.Y.Z (SHA) kit line naming another commit" '/^# kit:/ { print "# kit: proved-port v0.1.0 (0123456789abcdef)"; next } { print }'
bad verus/RLIMIT "a vX.Y.Z (SHA) kit line with too short a sha" "/^# kit:/ { print \"# kit: proved-port v0.1.0 ($(echo "$sha" | cut -c1-6))\"; next } { print }"
bad verus/REACH "no kit line" '/^# kit:/ { next } { print }'
headers || fail "headers.sh failed after the edits were undone"

# Once the kit commit is tagged v0.1.0 and the port re-syncs, KIT records
# the tag: the vX.Y.Z (SHA) lines pass only with that version, and the
# tag alone passes as one word.
git -C "$kitrepo" -c tag.gpgsign=false tag v0.1.0
sh "$kitrepo/kit/sync.sh" "$port" > /dev/null
grep -qx 'tag v0.1.0' "$port/verus/count/KIT" || fail "sync recorded no tag v0.1.0 in KIT"
headers || { cat "$tmp/out" >&2; fail "headers.sh failed on vX.Y.Z (SHA) lines naming KIT's tag"; }
echo "ok headers.sh passes vX.Y.Z (SHA) lines naming KIT's tag and commit"
cp "$port/verus/REACH" "$tmp/keep"
awk '/^# kit:/ { print "# kit: proved-port v0.1.0"; next } { print }' "$tmp/keep" > "$port/verus/REACH"
headers || { cat "$tmp/out" >&2; fail "headers.sh failed on a one-word kit line naming KIT's tag"; }
cp "$tmp/keep" "$port/verus/REACH"
echo "ok headers.sh passes a one-word kit line naming KIT's tag"
bad verus/RLIMIT "a vX.Y.Z (SHA) kit line naming another version than KIT's tag" "/^# kit:/ { print \"# kit: proved-port v0.0.9 ($sha)\"; next } { print }"
bad verus/RLIMIT "a vX.Y.Z (SHA) kit line naming KIT's tag and another commit" '/^# kit:/ { print "# kit: proved-port v0.1.0 (0123456789abcdef)"; next } { print }'
headers || fail "headers.sh failed after the tagged edits were undone"

# chainz's ratchet is verus/SHARES (VERUS_FLOOR): make verus-headers holds
# its header to the verus/FLOOR template's, and fails when it drifts.
mv "$port/verus/FLOOR" "$port/verus/SHARES"
# floor_headers: make verus-headers with VERUS_FLOOR=verus/SHARES.
floor_headers() {
  rc=0
  make -s -C "$port" verus-headers VERUS_FLOOR=verus/SHARES > "$tmp/out" 2>&1 || rc=$?
  return $rc
}
floor_headers || { cat "$tmp/out" >&2; fail "make verus-headers VERUS_FLOOR=verus/SHARES failed"; }
grep -qx 'ok verus/SHARES' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "make verus-headers VERUS_FLOOR=verus/SHARES: got = no ok verus/SHARES, want = it"; }
echo "ok make verus-headers checks VERUS_FLOOR (verus/SHARES)"
cp "$port/verus/SHARES" "$tmp/keep"
awk 'NR == 2 { print "# something the kit never said"; next } { print }' "$tmp/keep" > "$port/verus/SHARES"
if floor_headers; then
  cat "$tmp/out" >&2
  fail "make verus-headers VERUS_FLOOR=verus/SHARES: got = success on a drifted verus/SHARES, want = failure"
fi
grep -q 'headers: verus/SHARES: the header drifted' "$tmp/out" ||
  { cat "$tmp/out" >&2; fail "make verus-headers did not name verus/SHARES"; }
echo "ok make verus-headers fails a drifted VERUS_FLOOR (verus/SHARES)"
cp "$tmp/keep" "$port/verus/SHARES"

# --- the verus checks' base ------------------------------------------------
# Both verus checks run the same steps (verus-main's `steps: *verus-steps`),
# and the proofs step passes CHAINGUARD_BASE_REVISION as VERUS_BASE, so on
# main the base is the start of the settled push, not HEAD^; with the
# variable unset, the step fails before make runs.
yaml=kit/templates/.chainguard/ci/verus.yaml
# check_block NAME: the lines of check NAME in verus.yaml.
check_block() {
  awk -v name="  $1:" '$0 == name { on = 1; next } on && /^  [a-z#]/ { exit } on { print }' "$yaml"
}
check_block verus | grep -q '^    steps: &verus-steps$' || fail "verus.yaml: verus does not define &verus-steps"
[ "$(check_block verus-main | grep -c '^    steps:')" -eq 1 ] || fail "verus.yaml: verus-main has no steps line"
check_block verus-main | grep -q '^    steps: \*verus-steps$' \
  || { check_block verus-main >&2; fail "verus.yaml: verus-main does not run verus's steps (steps: *verus-steps)"; }
# The proofs step's shell command, the list item after its `- -c`.
proofs_cmd=$(check_block verus | awk '/^          - -c$/ { on = 1; next } on && /^          - / { sub(/^          - /, ""); print; exit }')
# shellcheck disable=SC2016  # the literal text of the step's command
base_arg='VERUS_BASE="${CHAINGUARD_BASE_REVISION:?'
case $proofs_cmd in
  *"$base_arg"*) ;;
  *) fail "verus.yaml: the proofs step does not pass CHAINGUARD_BASE_REVISION as VERUS_BASE: $proofs_cmd" ;;
esac
mkdir "$tmp/stubmake"
printf '#!/bin/sh\necho "make $*" >> "%s"\n' "$tmp/stubmake.log" > "$tmp/stubmake/make"
chmod +x "$tmp/stubmake/make"
: > "$tmp/stubmake.log"
PATH=$tmp/stubmake:$PATH CG_CPUS=8 CHAINGUARD_BASE_REVISION=0123abc sh -c "$proofs_cmd" \
  || fail "the verus proofs step, with a base: got = failure, want = success"
grep -qx 'make verus VERUS_THREADS=8 VERUS_BASE=0123abc' "$tmp/stubmake.log" \
  || { cat "$tmp/stubmake.log" >&2; fail "the verus proofs step: got = another make line, want = VERUS_BASE=0123abc"; }
: > "$tmp/stubmake.log"
if (unset CHAINGUARD_BASE_REVISION; PATH=$tmp/stubmake:$PATH sh -c "$proofs_cmd") 2> "$tmp/nobase.err"; then
  fail "the verus proofs step, no base: got = success, want = failure"
fi
[ ! -s "$tmp/stubmake.log" ] || { cat "$tmp/stubmake.log" >&2; fail "the verus proofs step, no base: got = make ran, want = it fails first"; }
grep -q 'no base revision to compare verus/FLOOR with' "$tmp/nobase.err" \
  || { cat "$tmp/nobase.err" >&2; fail "the verus proofs step, no base: the message does not say so"; }
echo "ok verus and verus-main pass CHAINGUARD_BASE_REVISION as VERUS_BASE, and fail without it"

echo "PASS"
