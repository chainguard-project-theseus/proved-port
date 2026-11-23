#!/bin/sh
# kit/kit-calls-test.sh: `make kit-calls` over the kit's own templates
# (the kit-calls-template check). It builds kit/checks/calls as
# proved-port-calls (go, no network: the standard library only), lays
# out a scratch PROVED_PORT_HOME as make-test.sh does, and fills the
# check files under kit/templates/.chainguard/ci with a sample port's
# values into a scratch port, with kit/templates/ci/kit-calls.rules as
# its ci/kit-calls.rules. Then, with the real chainctl:
#   - calls-test's TestTemplateCalls, which requires a call line for
#     each kit target the template's checks run, passes over explain's
#     output for the filled templates (with --update, that output is
#     written to its fixture, kit/checks/calls/testdata/templates.json,
#     first; a fixture that differs is a note, not a failure, since
#     another chainctl may print the same configuration differently;
#     the note carries a diff of the two split at JSON value
#     boundaries, at most 60 lines, and a changed copy of the fixture
#     checks that the diff names the changed value);
#   - make kit-calls passes, printing `ok` for every rule in the
#     template's rules file and nothing else;
#   - a source-rules that runs a forbidden copy (sh ci/heap-noalias.sh),
#     a kani with a when.changed list, and a verus whose first step is
#     no make step each fail it, naming why;
#   - a check file chainctl refuses fails it, with chainctl's
#     diagnostics and kit-calls' own line.
# No Verus, Kani or cargo runs. Run it from the repository root.
set -eu

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# --- the checker ------------------------------------------------------------
mkdir "$tmp/bin"
(cd kit/checks/calls && GOTOOLCHAIN=local GOFLAGS=-mod=mod go build -o "$tmp/bin/proved-port-calls" .) \
  || fail "go build kit/checks/calls"
PATH=$tmp/bin:$PATH
export PATH
command -v chainctl > /dev/null || fail "no chainctl on PATH"

# --- the scratch install ----------------------------------------------------
version=0.0.$$-kit-calls-test
home=$tmp/home
mkdir -p "$home/make" "$home/templates"
install -m644 kit/make/* "$home/make/"
cp -a kit/templates/. "$home/templates/"
printf '# kit: the proved-port kit, as kit-calls-test.sh installs it.\ncommit 0123456789abcdef0123456789abcdef01234567\ntag v%s\n' \
  "$version" > "$home/KIT"

# --- the scratch port: the template's check files, filled ------------------
port=$tmp/port
mkdir -p "$port/.chainguard/ci/env" "$port/ci"
# fill FILE: FILE with each placeholder the check files use replaced by a
# sample port's value.
fill() {
  awk '{
    gsub(/\{CAPI_CRATE\}/, "foo-capi"); gsub(/\{CLANG_STREAM\}/, "19")
    gsub(/\{CONSUMER_NAME\}/, "curl"); gsub(/\{KANI_RELEASE\}/, "0.66")
    gsub(/\{KIT_VERSION\}/, "0.4"); gsub(/\{LIBRARY\}/, "libfoo")
    gsub(/\{MSRV\}/, "1.85"); gsub(/\{PORT_ENV\}/, "dev"); gsub(/\{PORT\}/, "foo")
    gsub(/\{REFERENCE_PACKAGES\}/, "libfoo-dev"); gsub(/\{RUST_STREAM\}/, "rust-1.99")
    gsub(/\{SHELLCHECK_RELEASE\}/, "0.10"); gsub(/\{UPSTREAM_VERSION\}/, "1.0")
    gsub(/\{CHAINCTL_RELEASE\}/, "0.2.242")
    gsub(/\{VERUS_CRATES\}/, "foo"); gsub(/\{VERUS_RELEASE\}/, "0.2025")
    print
  }' "$1"
}
n=0
for f in kit/templates/.chainguard/ci/*.yaml kit/templates/.chainguard/ci/env/*.yaml; do
  rel=${f#kit/templates/}
  fill "$f" > "$port/$rel"
  n=$((n + 1))
done
[ "$n" -ge 8 ] || fail "filled $n check files, want at least 8"
# A placeholder fill() does not know would leave the file unfilled.
if grep -n '{[A-Z][A-Z0-9_]*}' "$port"/.chainguard/ci/*.yaml "$port"/.chainguard/ci/env/*.yaml; then
  fail "a placeholder above is not filled: add it to fill()"
fi
cp kit/templates/ci/kit-calls.rules "$port/ci/kit-calls.rules"
# shellcheck disable=SC2016  # make's $(PROVED_PORT_HOME), not the shell's
printf '%s\n' "PROVED_PORT_VERSION = $version" "PROVED_PORT_HOME ?= $home" \
  'include $(PROVED_PORT_HOME)/make/proved-port.mk' > "$port/Makefile"

# kitcalls: make kit-calls in the port; its output in $tmp/out.
kitcalls() {
  (cd "$port" && make --no-print-directory kit-calls) > "$tmp/out" 2>&1
}

# --- every kit target has a call line ----------------------------------------
# calls-test's TestTemplateCalls requires a call line for each kit target
# each template check's steps name, over its fixture. Here it runs over
# what this chainctl prints for the templates as they are now, so a
# fixture that lags them hides nothing. --update writes the fixture.
fixture=kit/checks/calls/testdata/templates.json
(cd "$port" && chainctl checks explain --output=json .) > "$tmp/explain.json" \
  || { cat "$tmp/explain.json" >&2; fail "chainctl checks explain over the filled templates"; }
if [ "${1-}" = --update ]; then
  cp "$tmp/explain.json" "$fixture"
  echo "wrote $fixture"
fi
(cd kit/checks/calls && KIT_CALLS_TEMPLATES=$tmp/explain.json GOTOOLCHAIN=local GOFLAGS=-mod=mod \
  go test -count=1 -run '^TestTemplateCalls$' -v .) > "$tmp/gotest" 2>&1 \
  || { cat "$tmp/gotest" >&2; fail "TestTemplateCalls over the templates' resolved configuration"; }
grep -qx -- '--- PASS: TestTemplateCalls.*' "$tmp/gotest" \
  || { cat "$tmp/gotest" >&2; fail "TestTemplateCalls did not run"; }
echo "ok every kit target a template check runs has a call line (TestTemplateCalls over this chainctl's output)"

# splitjson FILE: FILE with a line break after each `,` and around each
# `{`, `}`, `[` and `]`, so a diff of two one-line explain outputs shows
# the keys and values that differ. A `,` or bracket inside a string
# splits it too; this is a diagnostic only.
splitjson() {
  awk '{ gsub(/,/, ",\n"); gsub(/[[{]/, "&\n"); gsub(/[]}]/, "\n&"); print }' "$1"
}
# fixturenote GOT WANT: nothing when GOT and WANT are equal; otherwise a
# note line, then at most 60 lines of the unified split diff (- is WANT,
# the fixture; + is GOT, this chainctl's output), then a line counting
# the diff lines left out.
fixturenote() {
  cmp -s "$1" "$2" && return 0
  echo "note: $2 differs from this chainctl's output for the templates (another chainctl, or a template change); sh kit/kit-calls-test.sh --update rewrites it. Split diff (- fixture, + this chainctl):"
  splitjson "$2" > "$tmp/split-want"
  splitjson "$1" > "$tmp/split-got"
  diff -u "$tmp/split-want" "$tmp/split-got" > "$tmp/split-diff" || true
  head -n 60 "$tmp/split-diff" | awk '{ print "  " $0 }'
  more=$(($(wc -l < "$tmp/split-diff") - 60))
  [ "$more" -le 0 ] || echo "  ... $more more diff lines"
}
# The note names a value that differs: one changed value in a copy.
awk '{ sub(/"make"/, "\"kit-calls-test-changed\""); print }' "$fixture" > "$tmp/changed.json"
cmp -s "$tmp/changed.json" "$fixture" && fail "the fixture has no \"make\" value to change"
fixturenote "$tmp/changed.json" "$fixture" > "$tmp/note"
grep -q '^note: ' "$tmp/note" || { cat "$tmp/note" >&2; fail "a changed fixture: no note line"; }
grep -qx '  +"kit-calls-test-changed",\{0,1\}' "$tmp/note" \
  || { cat "$tmp/note" >&2; fail "a changed fixture: the note's diff does not show the changed value"; }
grep -qx '  -"make",\{0,1\}' "$tmp/note" \
  || { cat "$tmp/note" >&2; fail "a changed fixture: the note's diff does not show the fixture's value"; }
[ "$(wc -l < "$tmp/note")" -le 62 ] || { cat "$tmp/note" >&2; fail "a changed fixture: the note is longer than 62 lines"; }
# A large difference (every `"` changed) is cut to 60 diff lines and a count.
awk '{ gsub(/"/, "x"); print }' "$fixture" > "$tmp/large.json"
fixturenote "$tmp/large.json" "$fixture" > "$tmp/note"
splitjson "$fixture" > "$tmp/split-want"
splitjson "$tmp/large.json" > "$tmp/split-got"
total=$(diff -u "$tmp/split-want" "$tmp/split-got" | wc -l)
[ "$total" -gt 61 ] || fail "a large change: its split diff has $total lines, want more than 61"
[ "$(wc -l < "$tmp/note")" -eq 62 ] || { cat "$tmp/note" >&2; fail "a large change: the note is not 62 lines"; }
tail -n 1 "$tmp/note" | grep -qx "  \.\.\. $((total - 60)) more diff lines" \
  || { tail -n 3 "$tmp/note" >&2; fail "a large change: the last line does not count the $((total - 60)) diff lines left out"; }
[ -z "$(fixturenote "$fixture" "$fixture")" ] || fail "an equal fixture: fixturenote printed something"
echo "ok a fixture that differs prints a split diff naming the value, cut to 60 lines and a count"
fixturenote "$tmp/explain.json" "$fixture"

# --- the template passes ----------------------------------------------------
kitcalls || { cat "$tmp/out" >&2; fail "make kit-calls over the templates: got = failure, want = success"; }
awk '!/^[ \t]*(#|$)/ { print "ok " $0 }' kit/templates/ci/kit-calls.rules > "$tmp/want"
[ "$(wc -l < "$tmp/want")" -ge 10 ] || fail "the rules template holds fewer than 10 rules"
cmp -s "$tmp/out" "$tmp/want" || { diff "$tmp/want" "$tmp/out" >&2 || true; fail "make kit-calls over the templates: got != want (diff above)"; }
echo "ok make kit-calls holds every rule of the template over the template's checks"

# fails WHAT FILE AWK WANT: FILE (under the port's .chainguard/ci),
# rewritten by AWK, fails make kit-calls with a line holding WANT; then
# FILE is restored.
fails() {
  cp "$port/.chainguard/ci/$2" "$tmp/keep"
  awk "$3" "$tmp/keep" > "$port/.chainguard/ci/$2"
  if kitcalls; then
    cat "$tmp/out" >&2
    fail "make kit-calls, $1: got = success, want = failure"
  fi
  grep -qF -- "$4" "$tmp/out" || { cat "$tmp/out" >&2; fail "make kit-calls, $1: no line says $4"; }
  cp "$tmp/keep" "$port/.chainguard/ci/$2"
  echo "ok make kit-calls fails $1"
}

fails "a source-rules that runs ci/heap-noalias.sh" lint.yaml \
  '{ print } /^        cmd: \[make, kit-calls\]$/ { print "      - name: old copy"; print "        cmd: [sh, ci/heap-noalias.sh]" }' \
  "FAIL forbid ci/heap-noalias.sh: check source-rules step 3 names ci/heap-noalias.sh"
fails "a kani with a when.changed list" verus.yaml \
  '{ print } /^  kani:$/ { print "    when:"; print "      changed: [\"src/**\"]" }' \
  "FAIL call kani kani: the check runs only when its when.changed paths change"
fails "a verus whose first step is no make step" verus.yaml \
  '/^    steps: &verus-steps$/ { print; print "      - name: first"; print "        cmd: [sh, -c, \"echo x > GNUmakefile\"]"; next } { print }' \
  "FAIL call verus verus-headers verus: "
fails "a check file chainctl refuses" lint.yaml \
  '{ print } /^  rustfmt:$/ { print "    when:"; print "      changed: []" }' \
  "check rustfmt: \`when.changed:\` declares no path globs"
grep -qx "kit-calls: chainctl checks explain --output=json . failed" "$tmp/out" \
  || { cat "$tmp/out" >&2; fail "make kit-calls, a refused check file: no kit-calls line says explain failed"; }

echo "PASS"
