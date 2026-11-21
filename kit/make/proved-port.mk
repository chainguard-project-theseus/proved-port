# proved-port.mk: the proof targets every proved port shares. The
# theseus-proved-port package installs it, with kani.sh, headers.sh,
# lint-unsafe.sh and VERUS-FILES, in /usr/share/proved-port/make/, the
# kit's templates in
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
#   PROVED_PORT_VERSION = 0.4.8
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
#   verify            every target VERIFY_TARGETS lists (by default verus
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

.PHONY: verus verus-proofs verus-count verus-count-test verus-headers kani kani-list kani-regular lint-unsafe verify

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
proved_port_phony = verus verus-proofs verus-count verus-count-test verus-headers kani kani-list kani-regular lint-unsafe
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
