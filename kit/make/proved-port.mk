# proved-port.mk: the proof targets every proved port shares. The
# theseus-proved-port package installs it, with kani.sh, headers.sh,
# lint-unsafe.sh, noalias.sh, codegen.sh, perf-budget.sh, perf-budget.rs,
# skip-summary.sh, glibc-max.sh, known-failures.sh, header-check.sh,
# abi-elf.sh, kani-quotes.sh, deviations.sh, ledger-active.sh,
# known-cites.sh and VERUS-FILES, in
# /usr/share/proved-port/make/, the kit's templates in
# /usr/share/proved-port/templates/, the kit's commit and tag in
# /usr/share/proved-port/KIT, and the counter as /usr/bin/verus-count.
# The port pins the package (theseus-proved-port~X.Y.Z) in its check and
# session environments, and carries no copy of the kit. (A port whose
# Makefile still includes verus/make/proved-port.mk runs a vendored copy
# of an earlier release, 0.2.4 or before, not this file.)
#
# A port's Makefile names the release it pins, sets the variables below
# that differ from their defaults, then includes this file, and runs make
# from its root:
#
#   VERUS_CRATES = pcre2-unicode pcre2-syntax pcre2 pcre2-capi
#   KANI_CRATES  = $(VERUS_CRATES)
#   PROVED_PORT_VERSION = 0.4.11
#   PROVED_PORT_HOME ?= /usr/share/proved-port
#   include $(PROVED_PORT_HOME)/make/proved-port.mk
#
# The targets run where make is invoked: the checks run them in the
# guest, and a development session runs them with the port's own
# wrapper (`make run CMD='make verus'`). A step the port runs before one
# of them is a prerequisite it adds in its Makefile, with no recipe
# (`verus: verus-scan`; challoc's `sh verus/seams.sh --check`).
#
# Targets:
#   verus             verus-proofs, then verus-count: the verus checks'
#                     command (.chainguard/ci/verus.yaml).
#   verus-proofs      cargo verus over VERUS_CRATES, one crate at a time
#                     (VERUS_JOBS), Verus's statistics
#                     (--time-expanded --output-json) into VERUS_STATS,
#                     after cleaning VERUS_CLEAN's earlier results so
#                     Verus checks them again and the statistics are this
#                     run's.
#   verus-count       the counter (VERUS_COUNT, the package's
#                     verus-count; nothing is built) over the port: the
#                     coverage floor (VERUS_FLOOR), with the rlimit
#                     headroom (verus/RLIMIT) from VERUS_STATS, and the
#                     comparison with VERUS_BASE, which verus and
#                     verus-count require (VERUS_BASE=none for none).
#   verus-count-test  passes: the package's own tests cover the counter,
#                     and nothing is built here. It stays a target for
#                     the ports' Makefiles and checks that name it.
#   verus-headers     headers.sh: the kit's Verus files against the
#                     installed templates (PROVED_PORT_HOME/templates):
#                     the headers of verus/FLOOR, RLIMIT, REACH, MINTS and
#                     count.toml, of .chainguard/ci/verus.yaml, miri.yaml
#                     and .chainguard/reviewers.yaml, and the kit rule
#                     of .chainguard/review-prompt.md; VERUS_FLOOR's
#                     header is the verus/FLOOR template's.
#   kani              kani.sh: the Kani harnesses, all of them or a shard,
#                     failing on any failure and when none run, unless
#                     KANI_NONE = yes declares the port has none yet.
#   kani-list         kani.sh --list: the harnesses, one `CRATE NAME FILE`
#                     a line (or this shard's, with KANI_SHARDS).
#   kani-regular      kani.sh one harness at a time in regular format
#                     with CBMC's --verbosity 9, failing on
#                     KANI_IGNORING_PATTERN (challoc's kani-ignoring).
#   lint-unsafe       lint-unsafe.sh: every crate but UNSAFE_CRATES
#                     forbids unsafe code, at its roots and on rustc's
#                     command line for every target, and with
#                     UNSAFE_WORKSPACE_LINTS = yes every member inherits
#                     a workspace lint that denies it.
#   noalias           noalias.sh: no inherent method of NOALIAS_TYPE
#                     takes a noalias &mut self in the release
#                     library's LLVM IR, for each build NOALIAS_BUILDS
#                     names (challoc's heap-noalias).
#   codegen           codegen.sh base: capture the base revision's
#                     release library (CHAINGUARD_BASE_REVISION, else
#                     CODEGEN_BASE) for each build CODEGEN_BUILDS names,
#                     compare this tree's with it function by function
#                     (fncmp), and print the verdicts as ::notice lines;
#                     exit 2 when a function changed (challoc's codegen
#                     checks).
#   codegen-capture   codegen.sh capture CODEGEN_DIR, and
#   codegen-compare   codegen.sh compare CODEGEN_DIR: the same in two
#                     steps, around a local change.
#   perf-budget       perf-budget.sh check: for each build
#                     PERF_BUDGET_BUILDS names, the hot exports'
#                     fast-path counts in the x86_64 release library
#                     against the build's table, after a self-test
#                     (challoc's perf-budget).
#   perf-budget-print perf-budget.sh print: the measured table lines.
#   skip-summary      skip-summary.sh: the test log SKIP_LOG's `SKIP
#                     <key>` lines against the expected-skips manifest
#                     SKIP_MANIFEST; a new skip fails, a gone one is a
#                     notice (challoc's skip-summary).
#   glibc-max         glibc-max.sh: the library GLIBC_MAX_LIB needs no
#                     GLIBC_ symbol version above GLIBC_MAX (challoc's
#                     glibc-max).
#   known-failures    known-failures.sh: after a failed test run, every
#                     failure in KNOWN_FAILURES_LOG must be listed in
#                     KNOWN_FAILURES_LIST (challoc's known-failures).
#   header-check      header-check.sh: the macros HEADER_CHECK_PREFIXES
#                     names in the port's headers HEADER_CHECK_HEADERS
#                     equal the C library's in HEADER_CHECK_UPSTREAM
#                     (the PCRE2 port's header-check).
#   abi-elf           abi-elf.sh: the shared library ABI_ELF_LIB's SONAME,
#                     needs, thread-locals, version nodes and exports
#                     are the ones its version script ABI_ELF_MAP
#                     declares (the PCRE2 port's abi checks 1 to 3).
#   kani-quotes       kani-quotes.sh: each Kani record in KANI_QUOTES_DIR
#                     still quotes, in its section 1, the source it
#                     names (under KANI_QUOTES_ROOTS).
#   deviations        deviations.sh: the failing keys of a run's results
#                     DEVIATIONS_RESULTS are exactly those the list
#                     DEVIATIONS_LIST holds, each citing an active entry
#                     of DEVIATIONS_LEDGER (the PCRE2 port's conformance
#                     ratchet).
#   known-cites       known-cites.sh: each row of the known-divergence
#                     table KNOWN_CITES_FILE cites an active entry of
#                     KNOWN_CITES_LEDGER or an open issue of
#                     KNOWN_CITES_REPO (the PCRE2 port's fuzz-diff
#                     known-citations check).
#   verify           every target VERIFY_TARGETS lists (by default verus
#                     kani), in order: everything the port's checks
#                     verify, as the one command a package build runs.

# The directory this file is in (the package's make/), for its scripts.
PROVED_PORT_MAKE := $(dir $(lastword $(MAKEFILE_LIST)))
# PROVED_PORT_HOME: the installed kit's root, which holds KIT and
# templates/ (and make/, which a port includes this file from).
PROVED_PORT_HOME ?= /usr/share/proved-port
# Exported, so that headers.sh reads the templates and KIT of the root
# the guards below read.
export PROVED_PORT_HOME
# PROVED_PORT_VERSION: the kit release the port pins
# (theseus-proved-port~X.Y.Z). Every kit target fails unless it is set
# and PROVED_PORT_HOME/KIT records `tag vX.Y.Z`; and verus and
# verus-count fail unless `$(VERUS_COUNT) --version` prints `verus-count`
# and the release KIT records. So a port that drops or misspells the pin,
# a stale install, and another counter earlier on PATH all fail.
PROVED_PORT_VERSION ?=

# --- Verus ---------------------------------------------------------------
# VERUS_CRATES: the packages cargo verus checks, each passed as -p. Empty
# checks cargo's default package selection (a one-crate port).
VERUS_CRATES ?=
# VERUS_MODE: `focus` checks VERUS_CRATES and not their dependencies;
# `verify` (challoc) checks every package cargo selects.
VERUS_MODE ?= focus
# VERUS_CARGO_FLAGS: further cargo flags (the Expat port: `--features verus`;
# challoc's second quantum: `--no-default-features --features quantum8`).
VERUS_CARGO_FLAGS ?=
# VERUS_THREADS: Verus's --num-threads; empty leaves Verus's default (the
# host's available parallelism). The verus checks pass CG_CPUS.
VERUS_THREADS ?=
# VERUS_JOBS: cargo's -j for cargo verus; empty leaves cargo's default
# (as many jobs as the host has CPUs). cargo re-prints each crate's
# output a line at a time, so two crates Verus checks at once mix their
# pretty-printed --output-json documents in VERUS_STATS, and the counter
# refuses the file. 1 checks one crate at a time, and dependencies build
# one at a time too (a port with eight verified crates, cold: 77 s, 45 s
# with cargo's default); Verus still runs VERUS_THREADS SMT threads in
# each crate. Only a port whose cargo verus checks one crate sets it
# empty.
VERUS_JOBS ?= 1
# VERUS_FLAGS: further Verus flags, after the statistics flags.
VERUS_FLAGS ?=
# VERUS_STATS: where verus-proofs writes Verus's statistics and
# verus-count reads them.
VERUS_STATS ?= target/verus-stats.json
# VERUS_CLEAN: the packages whose earlier results verus-proofs cleans
# before cargo verus runs; by default VERUS_CRATES, and with neither set
# the packages cargo selects with no -p: the workspace's default members
# (`workspace_default_members` in `cargo metadata`), which are the root
# package alone when the root manifest is a package and has no
# default-members, and every member, or the listed default-members, when
# it is a virtual manifest. cargo keeps a package's result when
# nothing it fingerprints changed, and then Verus does not run and
# VERUS_STATS is empty, which the rlimit headroom check fails; a touched
# source does not count as a change. A port whose cargo verus
# checks packages beyond VERUS_CRATES or cargo's default selection
# names every package Verus checks here: in
# VERUS_MODE=verify, the verified workspace packages the selected ones
# depend on (cargo verus verify checks dependencies too), or a
# `--workspace` in VERUS_CARGO_FLAGS. Other dependencies, vstd among
# them, are kept.
VERUS_CLEAN ?= $(VERUS_CRATES)
# VERUS_TARGET_DIR: cargo's target directory. cargo verus keeps focus
# mode's results apart, under its verus-partial/, and verify mode's in it.
# A port whose VERUS_CARGO_FLAGS names another workspace's manifest
# (--manifest-path) sets it to that workspace's target directory, the
# one cargo verus builds in, so the clean removes the results there.
VERUS_TARGET_DIR ?= $(or $(CARGO_TARGET_DIR),target)
# VERUS_FLOOR: the ratchet file, the counter's --floor. It is the
# `[floor] path` of verus/count.toml too (verus/SHARES, say, in a port
# with a verus/floor/ directory).
VERUS_FLOOR ?= verus/FLOOR
# VERUS_RLIMIT: non-empty passes --rlimit VERUS_STATS, which holds each
# verified function to verus/RLIMIT; empty leaves it out, for a port
# with no verus/RLIMIT yet. Once the base revision has verus/RLIMIT, the
# counter fails a run against that base without it.
VERUS_RLIMIT ?= yes
# VERUS_BASE: the revision verus/FLOOR, verus/RLIMIT and the counter's
# other ratchets compare with (--base-rev). The verus checks pass the
# base Checks sets: the pull request's, or on main the start of the
# push the run covers. Empty fails verus and
# verus-count before they run anything, so a check that loses its base
# fails rather than skip the comparison. `none` runs the counter with
# --no-base, no comparison: for a local or development run
# (`make verus VERUS_BASE=none`) and the proof PR that adds the first
# FLOOR, never for a check once FLOOR is on the base.
VERUS_BASE ?=
# VERUS_COUNT_FLAGS: further counter flags (--list, --config FILE).
VERUS_COUNT_FLAGS ?=
# VERUS_COUNT: the counter to run, with no build step: the package's
# verus-count, from PATH, unless the port names another. Whichever it
# is, verus and verus-count check its --version against KIT.
VERUS_COUNT ?= verus-count

# The profile and targets VERUS_CARGO_FLAGS builds with (`-r`,
# `--release`, `--profile P`, `--target T`, each value form also as
# `--flag=V`): cargo clean -p removes one profile's artifacts, the
# default dev profile's, and one layout, the host's, unless told others.
# cargo verus passes these flags on to cargo.
verus_after = $(if $(word 2,$2),$(if $(filter $1,$(firstword $2)),$1 $(word 2,$2)) $(call verus_after,$1,$(wordlist 2,$(words $2),$2)))
verus_profile_flags = $(filter -r --release --profile=% --target=%,$(VERUS_CARGO_FLAGS)) \
  $(call verus_after,--profile,$(VERUS_CARGO_FLAGS)) \
  $(call verus_after,--target,$(VERUS_CARGO_FLAGS))
# The manifest VERUS_CARGO_FLAGS points cargo verus at (`--manifest-path
# P`, also as `--manifest-path=P`): the clean and its cargo metadata read
# the same workspace, which need not be the one where make runs.
verus_manifest_flags = $(filter --manifest-path=%,$(VERUS_CARGO_FLAGS)) \
  $(call verus_after,--manifest-path,$(VERUS_CARGO_FLAGS))
# With VERUS_CLEAN empty, the clean reads the default members' package
# IDs from `cargo metadata`'s one-line JSON (an ID holds no `"`, `,` or
# `]`), and fails, before it cleans anything, when it finds none: a
# `cargo clean` with no -p would remove the whole target directory.
verus_clean_none = verus-proofs: cargo metadata names no workspace default members to clean: set VERUS_CLEAN to the packages cargo verus checks
verus_clean_members = members=$$(cargo metadata --locked --no-deps --format-version 1 $(verus_manifest_flags) \
  | sed -n 's/.*"workspace_default_members":\[\([^]]*\)\].*/\1/p' | tr -d '"' | tr ',' ' ') \
  && [ -n "$$members" ] || { echo '$(verus_clean_none)' >&2; exit 1; };
# cargo writes CACHEDIR.TAG only into a target directory it creates, and
# refuses to clean one without it ("missing or invalid `CACHEDIR.TAG`").
# The recipe can create VERUS_TARGET_DIR before cargo does (the default
# VERUS_STATS is in it), so it writes cargo's tag there itself: else a
# fresh verify-mode checkout, whose clean is that directory, fails, and
# a clean skipped instead would leave a warm rerun empty statistics.
# The tag's text is cargo's own, all three lines.
verus_cachedir_tag = 'Signature: 8a477f597d28d172789f06886806bc55' \
  '\# This file is a cache directory tag created by cargo.' \
  '\# For information about cache directory tags see https://bford.info/cachedir/'
verus_tag_cmd = [ -f $(VERUS_TARGET_DIR)/CACHEDIR.TAG ] || { mkdir -p $(VERUS_TARGET_DIR) \
  && printf '%s\n' $(verus_cachedir_tag) > $(VERUS_TARGET_DIR)/CACHEDIR.TAG; }
verus_clean_cmd = $(if $(VERUS_CLEAN),,$(verus_clean_members) )cargo clean --locked \
  $(if $(VERUS_CLEAN),$(addprefix -p ,$(VERUS_CLEAN)),$$(printf ' -p %s' $$members)) \
  $(verus_manifest_flags) $(verus_profile_flags) \
  --target-dir $(VERUS_TARGET_DIR)$(if $(filter focus,$(VERUS_MODE)),/verus-partial)
# cargo verus refuses a -j before a -p ("Args forwarded to Cargo must
# precede args forwarded to Verus"), so -j comes last.
verus_proofs_cmd = cargo verus $(VERUS_MODE) --locked \
  $(addprefix -p ,$(VERUS_CRATES)) $(VERUS_CARGO_FLAGS) \
  $(if $(VERUS_JOBS),-j $(VERUS_JOBS)) \
  -- --time-expanded --output-json \
  $(if $(VERUS_THREADS),--num-threads $(VERUS_THREADS)) $(VERUS_FLAGS) \
  > $(VERUS_STATS)
verus_count_cmd = $(VERUS_COUNT) --floor $(VERUS_FLOOR) \
  $(if $(VERUS_RLIMIT),--rlimit $(VERUS_STATS)) \
  $(if $(filter none,$(VERUS_BASE)),--no-base,--base-rev $(VERUS_BASE)) $(VERUS_COUNT_FLAGS) .
# Expanded with the recipe, so it fails `make verus` (and `make -n
# verus`) before the proofs run, and leaves other targets alone.
verus_base_missing = no VERUS_BASE to compare $(VERUS_FLOOR) with: give the revision (the verus checks pass the base Checks sets, CHAINGUARD_BASE_REVISION), or VERUS_BASE=none for a local run or the proof PR that adds the first FLOOR
verus_base_check = $(if $(VERUS_BASE),,$(error $(verus_base_missing)))
# PROVED_PORT_VERSION must be set, and the release PROVED_PORT_HOME/KIT
# records must be that one. Expanded with each kit target's recipe, so a
# missing pin or a mismatch fails it (and `make -n` of it) before any
# command runs.
proved_port_kit_tag = $(shell sed -n 's/^tag v//p' $(PROVED_PORT_HOME)/KIT 2>/dev/null)
proved_port_kit_got = $(if $(proved_port_kit_tag),tag v$(proved_port_kit_tag),no tag)
proved_port_version_unset = PROVED_PORT_VERSION is not set: the port's Makefile names the kit release it pins (theseus-proved-port~X.Y.Z) as PROVED_PORT_VERSION = X.Y.Z before the include ($(PROVED_PORT_HOME)/KIT records $(proved_port_kit_got))
proved_port_version_mismatch = PROVED_PORT_VERSION is $(PROVED_PORT_VERSION), but $(PROVED_PORT_HOME)/KIT records $(proved_port_kit_got): install theseus-proved-port~$(PROVED_PORT_VERSION), or move the port's pin and PROVED_PORT_VERSION together
proved_port_version_check = $(if $(PROVED_PORT_VERSION),$(if $(filter $(PROVED_PORT_VERSION),$(proved_port_kit_tag)),,$(error $(proved_port_version_mismatch))),$(error $(proved_port_version_unset)))
# For the counter verus and verus-count run: its --version must name the
# release KIT records, whatever the pin says.
proved_port_count_got = $(strip $(shell $(VERUS_COUNT) --version 2>/dev/null))
proved_port_count_want = verus-count $(proved_port_kit_tag)
proved_port_count_mismatch = $(PROVED_PORT_HOME)/KIT records $(proved_port_kit_got), but `$(VERUS_COUNT) --version` prints $(or $(proved_port_count_got),nothing): put the package's verus-count first on PATH, or give VERUS_COUNT the installed counter
proved_port_count_check = $(if $(and $(proved_port_kit_tag),$(findstring $(proved_port_count_want),$(proved_port_count_got)),$(findstring $(proved_port_count_got),$(proved_port_count_want))),,$(error $(proved_port_count_mismatch)))

define verus_proofs_recipe
$(proved_port_version_check)
$(verus_tag_cmd)
mkdir -p $(dir $(VERUS_STATS))
$(verus_clean_cmd)
$(verus_proofs_cmd)
endef

define verus_count_recipe
$(proved_port_version_check)
$(proved_port_count_check)
$(verus_base_check)
$(verus_count_cmd)
endef

# --- Kani ----------------------------------------------------------------
# kani.sh reads these from its environment; its header says what each
# does. KANI_ARGS go to every cargo kani call. A port with no harnesses
# yet sets `KANI_NONE = yes` before the include, so `make kani` (and the
# kani checks) pass until its first harness, which must delete the line:
# kani.sh fails a run that finds harnesses under it, and an empty run
# without it.
KANI_NONE ?=
KANI_CRATES ?=
KANI_FLAGS ?=
KANI_LIST_FLAGS ?=
KANI_JOBS ?=
KANI_FORMAT ?=
KANI_CBMC_ARGS ?=
KANI_ONLY ?=
KANI_EXCLUDE ?=
KANI_HARNESS ?=
KANI_SHARD ?=
KANI_SHARDS ?=
KANI_WEIGHTS ?=
KANI_FAIL_PATTERN ?=
KANI_VERSION ?=
KANI_ARGS ?=
# KANI_MANIFEST: the workspace manifest kani.sh points cargo at, for a
# port whose workspace is not at its root (a uutils port's crates/uutils).
# It defaults to the `--manifest-path` VERUS_CARGO_FLAGS gives cargo
# verus, so one setting covers both; empty runs cargo in make's
# directory, as before. The recipes pass it to kani.sh when set.
KANI_MANIFEST ?= $(lastword $(patsubst --manifest-path=%,%,$(verus_manifest_flags)))
kani_env = $(if $(strip $(KANI_MANIFEST)),KANI_MANIFEST='$(strip $(KANI_MANIFEST))' )
export KANI_CRATES KANI_FLAGS KANI_LIST_FLAGS KANI_JOBS KANI_FORMAT KANI_CBMC_ARGS \
  KANI_ONLY KANI_EXCLUDE KANI_HARNESS KANI_SHARD KANI_SHARDS KANI_WEIGHTS \
  KANI_FAIL_PATTERN KANI_VERSION KANI_NONE

.PHONY: verus verus-proofs verus-count verus-count-test verus-headers kani kani-list kani-regular lint-unsafe noalias codegen codegen-capture codegen-compare perf-budget perf-budget-print skip-summary glibc-max known-failures header-check abi-elf kani-quotes deviations known-cites verify

verus: ## the Verus proofs, then the coverage floor and rlimit headroom (VERUS_BASE=<rev> or none)
	$(verus_proofs_recipe)
	$(verus_count_recipe)

verus-proofs: ## cargo verus over VERUS_CRATES, statistics to VERUS_STATS
	$(verus_proofs_recipe)

verus-count: ## the kit counter over the port, from VERUS_STATS (VERUS_BASE=<rev> or none)
	$(verus_count_recipe)

verus-count-test: ## passes: the package's own tests cover the counter, and nothing is built
	$(proved_port_version_check)
	@echo 'verus-count-test: $(VERUS_COUNT) is not built here; its package tests it'

verus-headers: ## the kit's Verus files against the installed templates (headers.sh)
	$(proved_port_version_check)
	VERUS_FLOOR=$(VERUS_FLOOR) sh $(PROVED_PORT_MAKE)headers.sh .

kani: ## the Kani harnesses, all or a shard (kani.sh; KANI_SHARD/KANI_SHARDS; KANI_NONE=yes for none yet)
	$(proved_port_version_check)
	$(kani_env)sh $(PROVED_PORT_MAKE)kani.sh $(KANI_ARGS)

kani-list: ## the Kani harnesses, one CRATE NAME FILE a line
	$(proved_port_version_check)
	$(kani_env)sh $(PROVED_PORT_MAKE)kani.sh --list $(KANI_ARGS)

# KANI_IGNORING_PATTERN: what kani-regular fails on. CBMC prints
# `warning: ignoring` when it leaves an expression unconstrained (Kani
# 0.68.0 with CBMC 6.11.0 does so for `<`, `<=`, `>` and `>=` on two
# non-constant bools), and the harness can then pass vacuously; the terse
# output KANI_JOBS implies hides the line. It becomes kani.sh's
# KANI_FAIL_PATTERN, a grep -E pattern, inside single quotes. Empty
# fails kani-regular: kani.sh reads an empty pattern as none, and the
# run would pass on the very line it exists to catch.
#
# kani-regular also fails a KANI_CBMC_ARGS that sets --verbosity. CBMC
# 6.11.0 takes an option's first value (cmdlinet::value_opt), and Kani
# 0.68.0 puts --cbmc-args before its own --verbosity 9, so a port's lower
# level would win, hide the warning and let the scan pass.
KANI_IGNORING_PATTERN ?= warning: ignoring
kani_ignoring_empty = KANI_IGNORING_PATTERN is empty: kani-regular fails on that pattern, and an empty one would scan for nothing (by default warning: ignoring)
kani_cbmc_verbosity = KANI_CBMC_ARGS sets --verbosity: kani-regular runs CBMC at --verbosity 9, and CBMC takes the first --verbosity, so a lower one would hide KANI_IGNORING_PATTERN's line

kani-regular: ## every Kani harness one at a time in regular format with CBMC's --verbosity 9, failing on KANI_IGNORING_PATTERN
	$(proved_port_version_check)
	$(if $(strip $(KANI_IGNORING_PATTERN)),,$(error $(kani_ignoring_empty)))
	$(if $(findstring --verbosity,$(KANI_CBMC_ARGS)),$(error $(kani_cbmc_verbosity)))
	KANI_JOBS= KANI_FORMAT=regular KANI_CBMC_ARGS='$(strip $(KANI_CBMC_ARGS) --verbosity 9)' KANI_FAIL_PATTERN='$(KANI_IGNORING_PATTERN)' $(kani_env)sh $(PROVED_PORT_MAKE)kani.sh $(KANI_ARGS)

# --- unsafe code -----------------------------------------------------------
# lint-unsafe.sh reads these from its environment; its header says what
# each does. UNSAFE_CRATES names the packages allowed unsafe code (the C
# API crate, and any other fence crate), and lint-unsafe fails while it
# is empty. A port whose manifests have no [workspace.lints.rust] sets
# UNSAFE_WORKSPACE_LINTS = no. UNSAFE_MANIFEST defaults to KANI_MANIFEST
# (from VERUS_CARGO_FLAGS' --manifest-path), so one setting covers a
# workspace that is not at the port's root.
UNSAFE_CRATES ?=
UNSAFE_WORKSPACE_LINTS ?= yes
UNSAFE_MANIFEST ?= $(KANI_MANIFEST)
UNSAFE_CHECK_FLAGS ?=
unsafe_crates_empty = UNSAFE_CRATES is empty: the port's Makefile names the packages allowed unsafe code (the C API crate)

lint-unsafe: ## every crate but UNSAFE_CRATES forbids unsafe code: roots, workspace lints, cargo check with -Funsafe_code (lint-unsafe.sh)
	$(proved_port_version_check)
	$(if $(strip $(UNSAFE_CRATES)),,$(error $(unsafe_crates_empty)))
	UNSAFE_CRATES='$(strip $(UNSAFE_CRATES))' UNSAFE_WORKSPACE_LINTS='$(strip $(UNSAFE_WORKSPACE_LINTS))' UNSAFE_MANIFEST='$(strip $(UNSAFE_MANIFEST))' UNSAFE_CHECK_FLAGS='$(strip $(UNSAFE_CHECK_FLAGS))' sh $(PROVED_PORT_MAKE)lint-unsafe.sh .

# --- noalias ---------------------------------------------------------------
# noalias.sh reads these from its environment; its header says what
# each does. NOALIAS_CRATE is the library's crate name and NOALIAS_TYPE
# the type whose `&mut self` must not be noalias, with its module path
# (heap::Heap); noalias fails while either is empty. NOALIAS_BUILDS
# names the builds (by default one, default), and NOALIAS_FLAGS_<name>
# holds a build's own cargo flags, so challoc's two quanta are
#   NOALIAS_BUILDS = q16 q8
#   NOALIAS_FLAGS_q8 = --no-default-features --features quantum8
# NOALIAS_CARGO_FLAGS go to every build.
NOALIAS_CRATE ?=
NOALIAS_TYPE ?=
NOALIAS_BUILDS ?= default
NOALIAS_CARGO_FLAGS ?=
noalias_crate_empty = NOALIAS_CRATE is empty: the port's Makefile names its library's crate
noalias_type_empty = NOALIAS_TYPE is empty: the port's Makefile names the type, with its module path (heap::Heap)
noalias_builds_empty = NOALIAS_BUILDS is empty: the port's Makefile names its builds (by default default)

noalias: ## no method of NOALIAS_TYPE takes a noalias &mut self in the release library's LLVM IR, per build in NOALIAS_BUILDS (noalias.sh)
	$(proved_port_version_check)
	$(if $(strip $(NOALIAS_CRATE)),,$(error $(noalias_crate_empty)))
	$(if $(strip $(NOALIAS_TYPE)),,$(error $(noalias_type_empty)))
	$(if $(strip $(NOALIAS_BUILDS)),,$(error $(noalias_builds_empty)))
	NOALIAS_CRATE='$(strip $(NOALIAS_CRATE))' NOALIAS_TYPE='$(strip $(NOALIAS_TYPE))' NOALIAS_BUILDS='$(strip $(NOALIAS_BUILDS))' $(foreach b,$(NOALIAS_BUILDS),NOALIAS_FLAGS_$(b)='$(strip $(NOALIAS_FLAGS_$(b)))' )NOALIAS_CARGO_FLAGS='$(strip $(NOALIAS_CARGO_FLAGS))' NOALIAS_TARGET_DIR='$(or $(CARGO_TARGET_DIR),target)' sh $(PROVED_PORT_MAKE)noalias.sh .

# --- codegen ---------------------------------------------------------------
# codegen.sh reads these from its environment; its header says what
# each does. CODEGEN_LIB is the release library's file name
# (libchalloc.so); every codegen target fails while it is empty.
# CODEGEN_BUILDS names the builds (by default one, default), and
# CODEGEN_FLAGS_<name> holds a build's own cargo flags, so challoc's two
# quanta are
#   CODEGEN_BUILDS = q16 q8
#   CODEGEN_FLAGS_q8 = --no-default-features --features quantum8
# CODEGEN_CARGO_FLAGS go to every build. CODEGEN_FNCMP is the comparer,
# the kit's kit/perf/codegen/fncmp built with go (by default fncmp, from
# PATH): the package does not install it. codegen compares with
# CHAINGUARD_BASE_REVISION (a check's base), else CODEGEN_BASE;
# codegen-capture and codegen-compare keep their capture in CODEGEN_DIR.
CODEGEN_LIB ?=
CODEGEN_BUILDS ?= default
CODEGEN_CARGO_FLAGS ?=
CODEGEN_FNCMP ?= fncmp
CODEGEN_BASE ?=
CODEGEN_DIR ?= target/codegen-baseline
codegen_lib_empty = CODEGEN_LIB is empty: the port's Makefile names its release library's file (libchalloc.so)
codegen_builds_empty = CODEGEN_BUILDS is empty: the port's Makefile names its builds (by default default)
codegen_dir_empty = CODEGEN_DIR is empty: name the directory of the capture (by default target/codegen-baseline)
codegen_env = CODEGEN_LIB='$(strip $(CODEGEN_LIB))' CODEGEN_BUILDS='$(strip $(CODEGEN_BUILDS))' $(foreach b,$(CODEGEN_BUILDS),CODEGEN_FLAGS_$(b)='$(strip $(CODEGEN_FLAGS_$(b)))' )CODEGEN_CARGO_FLAGS='$(strip $(CODEGEN_CARGO_FLAGS))' CODEGEN_TARGET_DIR='$(or $(CARGO_TARGET_DIR),target)' CODEGEN_FNCMP='$(strip $(CODEGEN_FNCMP))'
define codegen_check
$(proved_port_version_check)
$(if $(strip $(CODEGEN_LIB)),,$(error $(codegen_lib_empty)))
$(if $(strip $(CODEGEN_BUILDS)),,$(error $(codegen_builds_empty)))
endef

codegen: ## the release library's machine code from the base revision to this tree, function by function, per build in CODEGEN_BUILDS (codegen.sh base)
	$(codegen_check)
	$(codegen_env) CODEGEN_BASE='$(strip $(CODEGEN_BASE))' sh $(PROVED_PORT_MAKE)codegen.sh base

codegen-capture: ## save the release library's machine code in CODEGEN_DIR for codegen-compare (codegen.sh capture)
	$(codegen_check)
	$(if $(strip $(CODEGEN_DIR)),,$(error $(codegen_dir_empty)))
	$(codegen_env) sh $(PROVED_PORT_MAKE)codegen.sh capture '$(strip $(CODEGEN_DIR))'

codegen-compare: ## compare the release library's machine code with CODEGEN_DIR's capture, function by function (codegen.sh compare)
	$(codegen_check)
	$(if $(strip $(CODEGEN_DIR)),,$(error $(codegen_dir_empty)))
	$(codegen_env) sh $(PROVED_PORT_MAKE)codegen.sh compare '$(strip $(CODEGEN_DIR))'

# --- perf-budget -----------------------------------------------------------
# perf-budget.sh reads these from its environment; its header says what
# each does. PERF_BUDGET_LIB is the release library's file name
# (libchalloc.so); both targets fail while it is empty. PERF_BUDGET_BUILDS
# names the builds (by default one, default); each needs its table in
# PERF_BUDGET_TABLE_<name>, and PERF_BUDGET_FLAGS_<name> holds its own
# cargo flags, so challoc's two quanta are
#   PERF_BUDGET_BUILDS = q16 q8
#   PERF_BUDGET_TABLE_q16 = verus/perf-budget.txt
#   PERF_BUDGET_TABLE_q8 = verus/perf-budget-q8.txt
#   PERF_BUDGET_FLAGS_q8 = --no-default-features --features quantum8
# PERF_BUDGET_CARGO_FLAGS go to every build. PERF_BUDGET_SELFTEST_EXPORT
# names the export the self-test mutates (by default the table's first).
PERF_BUDGET_LIB ?=
PERF_BUDGET_BUILDS ?= default
PERF_BUDGET_CARGO_FLAGS ?=
PERF_BUDGET_SELFTEST_EXPORT ?=
perf_budget_lib_empty = PERF_BUDGET_LIB is empty: the port's Makefile names its release library's file (libchalloc.so)
perf_budget_builds_empty = PERF_BUDGET_BUILDS is empty: the port's Makefile names its builds (by default default)
perf_budget_env = PERF_BUDGET_LIB='$(strip $(PERF_BUDGET_LIB))' PERF_BUDGET_BUILDS='$(strip $(PERF_BUDGET_BUILDS))' $(foreach b,$(PERF_BUDGET_BUILDS),PERF_BUDGET_TABLE_$(b)='$(strip $(PERF_BUDGET_TABLE_$(b)))' PERF_BUDGET_FLAGS_$(b)='$(strip $(PERF_BUDGET_FLAGS_$(b)))' )PERF_BUDGET_CARGO_FLAGS='$(strip $(PERF_BUDGET_CARGO_FLAGS))' PERF_BUDGET_TARGET_DIR='$(or $(CARGO_TARGET_DIR),target)' PERF_BUDGET_SELFTEST_EXPORT='$(strip $(PERF_BUDGET_SELFTEST_EXPORT))'
define perf_budget_check
$(proved_port_version_check)
$(if $(strip $(PERF_BUDGET_LIB)),,$(error $(perf_budget_lib_empty)))
$(if $(strip $(PERF_BUDGET_BUILDS)),,$(error $(perf_budget_builds_empty)))
endef

perf-budget: ## the hot exports' fast-path counts in the x86_64 release library against each build's table, after a self-test (perf-budget.sh check)
	$(perf_budget_check)
	$(perf_budget_env) sh $(PROVED_PORT_MAKE)perf-budget.sh check

perf-budget-print: ## the measured fast-path table lines of each build, to update its table after a deliberate change (perf-budget.sh print)
	$(perf_budget_check)
	$(perf_budget_env) sh $(PROVED_PORT_MAKE)perf-budget.sh print

# --- skip-summary ----------------------------------------------------------
# skip-summary.sh LOG MANIFEST; its header says what it does. SKIP_LOG is
# the test run's log and SKIP_MANIFEST the expected-skips manifest it is
# held to (challoc's ci/expected-skips/<cell>.txt); the target fails while
# either is empty. CI_SKIP_UPDATE=1 in the environment writes the
# observed keys to SKIP_MANIFEST and passes.
SKIP_LOG ?=
SKIP_MANIFEST ?=
skip_log_empty = SKIP_LOG is empty: name the test run's log (make skip-summary SKIP_LOG=test.log)
skip_manifest_empty = SKIP_MANIFEST is empty: name the expected-skips manifest the log is held to (ci/expected-skips/<cell>.txt)

skip-summary: ## the SKIP lines of the test log SKIP_LOG against the expected-skips manifest SKIP_MANIFEST: a new skip fails (skip-summary.sh)
	$(proved_port_version_check)
	$(if $(strip $(SKIP_LOG)),,$(error $(skip_log_empty)))
	$(if $(strip $(SKIP_MANIFEST)),,$(error $(skip_manifest_empty)))
	sh $(PROVED_PORT_MAKE)skip-summary.sh '$(strip $(SKIP_LOG))' '$(strip $(SKIP_MANIFEST))'
# --- glibc-max -------------------------------------------------------------
# glibc-max.sh SO MAX; its header says what it does. GLIBC_MAX_LIB is the
# built library to read (challoc's .ci/target/release/libchalloc.so) and
# GLIBC_MAX the highest GLIBC_ version it may need (2.44, Wolfi's glibc
# stream); the target fails while either is empty. The script runs
# binutils' readelf, which the port's check environment provides.
GLIBC_MAX_LIB ?=
GLIBC_MAX ?=
glibc_max_lib_empty = GLIBC_MAX_LIB is empty: name the built library to read (make glibc-max GLIBC_MAX_LIB=target/release/libfoo.so)
glibc_max_empty = GLIBC_MAX is empty: name the highest GLIBC_ version the library may need (GLIBC_MAX=2.44)

glibc-max: ## the library GLIBC_MAX_LIB needs no GLIBC_ symbol version above GLIBC_MAX (glibc-max.sh; needs binutils)
	$(proved_port_version_check)
	$(if $(strip $(GLIBC_MAX_LIB)),,$(error $(glibc_max_lib_empty)))
	$(if $(strip $(GLIBC_MAX)),,$(error $(glibc_max_empty)))
	sh $(PROVED_PORT_MAKE)glibc-max.sh '$(strip $(GLIBC_MAX_LIB))' '$(strip $(GLIBC_MAX))'
# --- known-failures --------------------------------------------------------
# known-failures.sh LOG LIST [EXTRA]; its header says what it does.
# KNOWN_FAILURES_LOG is the failed test run's log and KNOWN_FAILURES_LIST
# the failures it may have (challoc's ci/known-failures.txt); the target
# fails while either is empty. KNOWN_FAILURES_EXTRA, optional, is a
# port's sh script that prints more failure keys from the log (challoc's
# cgroup verdicts). CI_CELL in the environment names the cell for the
# list's cell-scoped entries. A test binary that crashed, before or
# after its result line, is a failure, `crashed: TARGET`; one that never
# started is not seen, so a port that
# needs every group to run checks that itself (as challoc's
# ci/test-cell.sh does). The log holds cargo's stderr too (`2>&1`).
KNOWN_FAILURES_LOG ?=
KNOWN_FAILURES_LIST ?=
KNOWN_FAILURES_EXTRA ?=
known_failures_log_empty = KNOWN_FAILURES_LOG is empty: name the failed test run's log (make known-failures KNOWN_FAILURES_LOG=test.log)
known_failures_list_empty = KNOWN_FAILURES_LIST is empty: name the list of failures the run may have (ci/known-failures.txt)

known-failures: ## after a failed test run, every failure in KNOWN_FAILURES_LOG must be listed in KNOWN_FAILURES_LIST (known-failures.sh)
	$(proved_port_version_check)
	$(if $(strip $(KNOWN_FAILURES_LOG)),,$(error $(known_failures_log_empty)))
	$(if $(strip $(KNOWN_FAILURES_LIST)),,$(error $(known_failures_list_empty)))
	sh $(PROVED_PORT_MAKE)known-failures.sh '$(strip $(KNOWN_FAILURES_LOG))' '$(strip $(KNOWN_FAILURES_LIST))'$(if $(strip $(KNOWN_FAILURES_EXTRA)), '$(strip $(KNOWN_FAILURES_EXTRA))')
# --- header-check ----------------------------------------------------------
# header-check.sh UPSTREAM PREFIXES HEADER...; its header says what it
# does. HEADER_CHECK_HEADERS are the port's public headers
# (include/foo.h include/foo-posix.h), each compared with the file of its
# name in HEADER_CHECK_UPSTREAM (the C library's installed headers, by
# default /usr/include, which the C library's -dev package in the port's
# check environment fills), for the macros named with a prefix of
# HEADER_CHECK_PREFIXES (FOO_ REG_); the target fails while the headers
# or the prefixes are empty.
HEADER_CHECK_HEADERS ?=
HEADER_CHECK_PREFIXES ?=
HEADER_CHECK_UPSTREAM ?= /usr/include
header_check_headers_empty = HEADER_CHECK_HEADERS is empty: name the port's public headers (make header-check HEADER_CHECK_HEADERS=include/foo.h)
header_check_prefixes_empty = HEADER_CHECK_PREFIXES is empty: name the prefixes of the C library's macros to compare (HEADER_CHECK_PREFIXES="FOO_ REG_")
header_check_upstream_empty = HEADER_CHECK_UPSTREAM is empty: name the C library's installed include directory (by default /usr/include)

header-check: ## the macros HEADER_CHECK_PREFIXES names in the port's headers HEADER_CHECK_HEADERS equal the C library's in HEADER_CHECK_UPSTREAM (header-check.sh)
	$(proved_port_version_check)
	$(if $(strip $(HEADER_CHECK_HEADERS)),,$(error $(header_check_headers_empty)))
	$(if $(strip $(HEADER_CHECK_PREFIXES)),,$(error $(header_check_prefixes_empty)))
	$(if $(strip $(HEADER_CHECK_UPSTREAM)),,$(error $(header_check_upstream_empty)))
	sh $(PROVED_PORT_MAKE)header-check.sh '$(strip $(HEADER_CHECK_UPSTREAM))' '$(strip $(HEADER_CHECK_PREFIXES))' $(foreach h,$(HEADER_CHECK_HEADERS),'$(h)')
# --- abi-elf ---------------------------------------------------------------
# abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]; its header says
# what it does.
# ABI_ELF_LIB is the built shared library (target/release/libfoo.so.1),
# ABI_ELF_MAP its linker version script (maps/libfoo.map) and
# ABI_ELF_SONAME the C library's SONAME; the target fails while any is
# empty. ABI_ELF_NEEDED lists the libraries it may need (by default libc
# and the x86_64 and aarch64 loaders), and ABI_ELF_TLS_MAX, optional,
# bounds its thread-locals in bytes (512, glibc's default optional static
# TLS). ABI_ELF_DEFAULTS, needed only for a name the map lists in more
# than one node, gives each such name's default version as the C
# library exports it (foo@V2 for its foo@@V2). Run it once per library.
# The script runs binutils' readelf, which the port's check environment
# provides.
ABI_ELF_LIB ?=
ABI_ELF_MAP ?=
ABI_ELF_SONAME ?=
ABI_ELF_NEEDED ?= libc.so.6 ld-linux-x86-64.so.2 ld-linux-aarch64.so.1
ABI_ELF_TLS_MAX ?=
ABI_ELF_DEFAULTS ?=
abi_elf_lib_empty = ABI_ELF_LIB is empty: name the built shared library (make abi-elf ABI_ELF_LIB=target/release/libfoo.so.1)
abi_elf_map_empty = ABI_ELF_MAP is empty: name the library's linker version script (ABI_ELF_MAP=maps/libfoo.map)
abi_elf_soname_empty = ABI_ELF_SONAME is empty: name the C library's SONAME (ABI_ELF_SONAME=libfoo.so.1)
abi_elf_needed_empty = ABI_ELF_NEEDED is empty: list the libraries the library may need (by default libc and the loaders)

abi-elf: ## the library ABI_ELF_LIB's SONAME, needs, thread-locals, version nodes and exports against its version script ABI_ELF_MAP (abi-elf.sh; needs binutils)
	$(proved_port_version_check)
	$(if $(strip $(ABI_ELF_LIB)),,$(error $(abi_elf_lib_empty)))
	$(if $(strip $(ABI_ELF_MAP)),,$(error $(abi_elf_map_empty)))
	$(if $(strip $(ABI_ELF_SONAME)),,$(error $(abi_elf_soname_empty)))
	$(if $(strip $(ABI_ELF_NEEDED)),,$(error $(abi_elf_needed_empty)))
	sh $(PROVED_PORT_MAKE)abi-elf.sh '$(strip $(ABI_ELF_LIB))' '$(strip $(ABI_ELF_MAP))' '$(strip $(ABI_ELF_SONAME))' '$(strip $(ABI_ELF_NEEDED))'$(if $(strip $(ABI_ELF_TLS_MAX) $(ABI_ELF_DEFAULTS)), '$(strip $(ABI_ELF_TLS_MAX))')$(if $(strip $(ABI_ELF_DEFAULTS)), '$(strip $(ABI_ELF_DEFAULTS))')
# --- kani-quotes -------------------------------------------------------------
# kani-quotes.sh DIR ROOTS [SKIP]; its header says what it does.
# KANI_QUOTES_DIR holds the port's Kani records (kani-*.md, by default
# verus/seams); KANI_QUOTES_ROOTS are the directories a record's source
# path is looked up under, in turn (by default the port's tree, `.`; add
# a vstd source tree for records that quote vstd's std_specs);
# KANI_QUOTES_SKIP names records it does not read (kani-s3.md). The
# target fails while the directory or the roots are empty.
KANI_QUOTES_DIR ?= verus/seams
KANI_QUOTES_ROOTS ?= .
KANI_QUOTES_SKIP ?=
kani_quotes_dir_empty = KANI_QUOTES_DIR is empty: name the directory of the port's Kani records (by default verus/seams)
kani_quotes_roots_empty = KANI_QUOTES_ROOTS is empty: name the directories a record's source is looked up under (by default .)

kani-quotes: ## each Kani record in KANI_QUOTES_DIR still quotes the source it names, in its section 1 (kani-quotes.sh)
	$(proved_port_version_check)
	$(if $(strip $(KANI_QUOTES_DIR)),,$(error $(kani_quotes_dir_empty)))
	$(if $(strip $(KANI_QUOTES_ROOTS)),,$(error $(kani_quotes_roots_empty)))
	sh $(PROVED_PORT_MAKE)kani-quotes.sh '$(strip $(KANI_QUOTES_DIR))' '$(strip $(KANI_QUOTES_ROOTS))'$(if $(strip $(KANI_QUOTES_SKIP)), '$(strip $(KANI_QUOTES_SKIP))')
# --- deviations --------------------------------------------------------------
# deviations.sh [--update] RESULTS LIST LEDGER; its header says what it
# does. DEVIATIONS_RESULTS is the port's run's `PASS KEY` / `FAIL KEY`
# file (the PCRE2 port's conformance run writes one, a key per pattern
# block), DEVIATIONS_LIST the deviations list it is held to
# (tests/upstream/deviations.txt), and DEVIATIONS_LEDGER the ABI ledger
# each reason cites (by default docs/abi, the kit template's directory;
# or one file of entries); the target fails while any is empty.
# DEVIATIONS_UPDATE=1 drops the listed keys that pass or are gone, then
# checks; any other non-blank value (0, no) fails the target, so a value
# meant as "off" never rewrites the list. DEVIATIONS_OUT, in the environment or on the command line,
# names a directory for the full lists.
DEVIATIONS_RESULTS ?=
DEVIATIONS_LIST ?=
DEVIATIONS_LEDGER ?= docs/abi
DEVIATIONS_UPDATE ?=
deviations_results_empty = DEVIATIONS_RESULTS is empty: name the run's PASS/FAIL results file (make deviations DEVIATIONS_RESULTS=target/conformance/results)
deviations_list_empty = DEVIATIONS_LIST is empty: name the deviations list the results are held to (tests/upstream/deviations.txt)
deviations_ledger_empty = DEVIATIONS_LEDGER is empty: name the ABI ledger the reasons cite (by default docs/abi)
deviations_update_bad = DEVIATIONS_UPDATE is '$(strip $(DEVIATIONS_UPDATE))': set it to 1 to drop the listed keys that pass or are gone, or leave it empty

deviations: ## the failing keys of DEVIATIONS_RESULTS are exactly those DEVIATIONS_LIST lists, each citing an active entry of DEVIATIONS_LEDGER (deviations.sh)
	$(proved_port_version_check)
	$(if $(strip $(DEVIATIONS_RESULTS)),,$(error $(deviations_results_empty)))
	$(if $(strip $(DEVIATIONS_LIST)),,$(error $(deviations_list_empty)))
	$(if $(strip $(DEVIATIONS_LEDGER)),,$(error $(deviations_ledger_empty)))
	$(if $(filter-out 1,$(strip $(DEVIATIONS_UPDATE))),$(error $(deviations_update_bad)))
	sh $(PROVED_PORT_MAKE)deviations.sh $(if $(strip $(DEVIATIONS_UPDATE)),--update )'$(strip $(DEVIATIONS_RESULTS))' '$(strip $(DEVIATIONS_LIST))' '$(strip $(DEVIATIONS_LEDGER))'

# --- known-cites -------------------------------------------------------------
# known-cites.sh FILE LEDGER REPO; its header says what it does.
# KNOWN_CITES_FILE is the known-divergence table's source (the PCRE2
# port's fuzz/diff/known.c), KNOWN_CITES_LEDGER the ABI ledger an
# `ABI-nnn` cite names (by default docs/abi, the kit template's
# directory; or one file of entries), and KNOWN_CITES_REPO the
# repository a `TODO(#n)` cite's issue is in, `owner/name`, then any
# names it had before it moved; the target fails while any is empty.
# KNOWN_CITES_MACRO names the row macro (empty: KNOWN). The pull
# request's description is read from KNOWN_CITES_PR_BODY in the
# environment, where the port's check binds it. An issue whose live
# state gh cannot check fails the target; KNOWN_CITES_OFFLINE=1 (a
# local run without gh) makes it a NOTE, and any other non-blank value
# fails the target. Both are passed to the script stripped, always: make
# exports a command-line value to the recipe's environment as given, and
# the script reads an empty one as unset.
KNOWN_CITES_OFFLINE ?=
KNOWN_CITES_FILE ?=
KNOWN_CITES_LEDGER ?= docs/abi
KNOWN_CITES_REPO ?=
KNOWN_CITES_MACRO ?=
known_cites_file_empty = KNOWN_CITES_FILE is empty: name the known-divergence table's source (make known-cites KNOWN_CITES_FILE=fuzz/diff/known.c)
known_cites_ledger_empty = KNOWN_CITES_LEDGER is empty: name the ABI ledger the ABI-nnn cites name (by default docs/abi)
known_cites_repo_empty = KNOWN_CITES_REPO is empty: name the repository the TODO cites' issues are in (owner/name)
known_cites_offline_bad = KNOWN_CITES_OFFLINE is '$(strip $(KNOWN_CITES_OFFLINE))': set it to 1 for a run without gh, or leave it empty

known-cites: ## each row of the known-divergence table KNOWN_CITES_FILE cites an active entry of KNOWN_CITES_LEDGER or an open issue of KNOWN_CITES_REPO (known-cites.sh)
	$(proved_port_version_check)
	$(if $(strip $(KNOWN_CITES_FILE)),,$(error $(known_cites_file_empty)))
	$(if $(strip $(KNOWN_CITES_LEDGER)),,$(error $(known_cites_ledger_empty)))
	$(if $(strip $(KNOWN_CITES_REPO)),,$(error $(known_cites_repo_empty)))
	$(if $(filter-out 1,$(strip $(KNOWN_CITES_OFFLINE))),$(error $(known_cites_offline_bad)))
	KNOWN_CITES_OFFLINE='$(strip $(KNOWN_CITES_OFFLINE))' KNOWN_CITES_MACRO='$(strip $(KNOWN_CITES_MACRO))' sh $(PROVED_PORT_MAKE)known-cites.sh '$(strip $(KNOWN_CITES_FILE))' '$(strip $(KNOWN_CITES_LEDGER))' '$(strip $(KNOWN_CITES_REPO))'

# --- verify ----------------------------------------------------------------
# VERIFY_TARGETS: every target the port's checks verify with, in the
# order `make verify` runs them, each in its own make. A package build
# of the port runs `make verify VERUS_BASE=none` and nothing else, so
# the list names everything the checks verify with Verus and Kani, and
# the controls those checks run, the port's own targets included:
# challoc's lists its quantum-8 proofs, its not-Copy control, its Kani
# bool-comparison scan, and kani and kani-regular at each quantum.
# Tests, Miri, fuzzing and other suites stay out: the package build has
# its own tests. A port that leaves Kani out sets `VERIFY_TARGETS =
# verus`.
#
# verify refuses a list that is empty, that names verify (which would
# run itself without end), or that names a file or directory in the
# port's root other than the kit's own targets, which are .PHONY (a
# port has a verus/ directory): make takes a target with no rule, or one
# not marked .PHONY, that has a file of its name as done, so the run
# would print the name, check nothing for it and pass. Each sub-make runs
# with make's built-in rules off (-r), so a listed name with no rule of
# the port's fails ("No rule to make target") even beside a NAME.sh or
# NAME.c a built-in rule would build it from.
VERIFY_TARGETS ?= verus kani
# The kit's targets, .PHONY above (verify aside).
proved_port_phony = verus verus-proofs verus-count verus-count-test verus-headers kani kani-list kani-regular lint-unsafe noalias codegen codegen-capture codegen-compare perf-budget perf-budget-print skip-summary glibc-max known-failures header-check abi-elf kani-quotes deviations known-cites
verify_targets_on_disk = $(wildcard $(filter-out $(proved_port_phony),$(VERIFY_TARGETS)))
verify_targets_empty = VERIFY_TARGETS is empty: the port's Makefile lists the targets its checks verify with (by default verus kani)
verify_targets_self = VERIFY_TARGETS names verify, which would run itself without end
verify_targets_files = VERIFY_TARGETS names $(verify_targets_on_disk), a file or directory in the port's root, which make could take as done and check nothing for: give the target another name

verify: ## every target VERIFY_TARGETS lists (by default verus kani), in order: what the port's checks verify, as one command
	$(proved_port_version_check)
	$(if $(strip $(VERIFY_TARGETS)),,$(error $(verify_targets_empty)))
	$(if $(filter verify,$(VERIFY_TARGETS)),$(error $(verify_targets_self)))
	$(if $(verify_targets_on_disk),$(error $(verify_targets_files)))
	@for t in $(VERIFY_TARGETS); do echo "verify: make $$t"; $(MAKE) -r --no-print-directory $$t || exit 1; done
