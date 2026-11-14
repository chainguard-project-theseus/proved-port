# proved-port.mk: the proof targets every proved port shares. The
# theseus-proved-port package installs it, with kani.sh, headers.sh and
# VERUS-FILES, in /usr/share/proved-port/make/, the kit's templates in
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
#   VERUS_CRATES = pcre3-unicode pcre3-syntax pcre3 pcre3-capi
#   KANI_CRATES  = $(VERUS_CRATES)
#   PROVED_PORT_VERSION = 0.3.0
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
#   verus-proofs      cargo verus over VERUS_CRATES, Verus's statistics
#                     (--time-expanded --output-json) into VERUS_STATS.
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
#                     and .chainguard/reviewers.yaml, and the vendored-kit
#                     rule of .chainguard/review-prompt.md; VERUS_FLOOR's
#                     header is the verus/FLOOR template's.
#   kani              kani.sh: the Kani harnesses, all of them or a shard,
#                     failing on any failure and when none run, unless
#                     KANI_NONE = yes declares the port has none yet.
#   kani-list         kani.sh --list: the harnesses, one `CRATE NAME FILE`
#                     a line (or this shard's, with KANI_SHARDS).

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
# VERUS_CARGO_FLAGS: further cargo flags (expat-rs: `--features verus`;
# challoc's second quantum: `--no-default-features --features quantum8`).
VERUS_CARGO_FLAGS ?=
# VERUS_THREADS: Verus's --num-threads; empty leaves Verus's default (the
# host's available parallelism). The verus checks pass CG_CPUS.
VERUS_THREADS ?=
# VERUS_FLAGS: further Verus flags, after the statistics flags.
VERUS_FLAGS ?=
# VERUS_STATS: where verus-proofs writes Verus's statistics and
# verus-count reads them.
VERUS_STATS ?= target/verus-stats.json
# VERUS_FLOOR: the ratchet file, the counter's --floor. It is the
# `[floor] path` of verus/count.toml too (verus/SHARES, say, in a port
# with a verus/floor/ directory).
VERUS_FLOOR ?= verus/FLOOR
# VERUS_RLIMIT: non-empty passes --rlimit VERUS_STATS, which holds each
# verified function to verus/RLIMIT; empty leaves it out, for a port
# with no verus/RLIMIT yet.
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

verus_proofs_cmd = cargo verus $(VERUS_MODE) --locked \
  $(addprefix -p ,$(VERUS_CRATES)) $(VERUS_CARGO_FLAGS) \
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
mkdir -p $(dir $(VERUS_STATS))
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
export KANI_CRATES KANI_FLAGS KANI_LIST_FLAGS KANI_JOBS KANI_FORMAT KANI_CBMC_ARGS \
  KANI_ONLY KANI_EXCLUDE KANI_HARNESS KANI_SHARD KANI_SHARDS KANI_WEIGHTS \
  KANI_FAIL_PATTERN KANI_VERSION KANI_NONE

.PHONY: verus verus-proofs verus-count verus-count-test verus-headers kani kani-list

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
	sh $(PROVED_PORT_MAKE)kani.sh $(KANI_ARGS)

kani-list: ## the Kani harnesses, one CRATE NAME FILE a line
	$(proved_port_version_check)
	sh $(PROVED_PORT_MAKE)kani.sh --list $(KANI_ARGS)
