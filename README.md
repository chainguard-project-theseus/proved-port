# proved-port

How to port a C library to Rust so that the Rust library loads in place
of the C one, is proved safe, is shown correct against the C original,
and is measured at least as fast.

This repository holds:

- **`plugins/proved-c-port`**, a Claude Code plugin with the
  `proved-c-port` skill: the lifecycle, proof discipline, ABI
  compatibility, performance method, landing rules, a sub-agent brief
  template and dated pitfalls.
- **`kit/`**, a starter kit of the tooling the method relies on
  (coverage counter and FLOOR ratchet, seam and theorem skeletons,
  differential and fuzz harnesses, bench scripts), extracted piece by
  piece as `kit/PLAN.md` lays out; the ports it came from consume it
  ("How a port consumes the kit", below). `kit/templates/` holds the
  document skeletons a new port copies.

The method is distilled from two ports:

- [chainguard-project-theseus/challoc](https://github.com/chainguard-project-theseus/challoc),
  a `malloc` replacement.
- pcre3, a PCRE2 10.49 replacement with a proved JIT on aarch64 and
  x86-64 (`wolfi-dev/pcre3`, not yet public).

## Install

Add the marketplace and install the plugin in Claude Code:

```
/plugin marketplace add chainguard-project-theseus/proved-port
/plugin install proved-c-port@proved-port
```

To try a local checkout instead, from the directory that holds it:

```
claude plugin marketplace add ./proved-port --scope local
claude plugin install proved-c-port@proved-port --scope local
```

The skill then loads on its own when a session works on porting a C
library to a Rust drop-in, writing Verus or Kani proofs for one,
building a differential harness against the C original, or measuring a
port against it.

To make it available to everyone working in a repository, add the
marketplace and plugin to that repository's `.claude/settings.json`,
naming this repository, `chainguard-project-theseus/proved-port`, as the
marketplace's GitHub source. A port's copy of
`kit/templates/.claude/settings.json` does this, with the marketplace's
`ref` at the release the port pins.

## How a port consumes the kit

- **The package.** A port pins the kit as the Wolfi package
  `theseus-proved-port~X.Y.Z` (v0.3.0 or later), in the packages of its
  verus checks and of `.chainguard/ci/env/dev.yaml`. The package
  installs the counter as `verus-count`, and the make targets, their
  scripts, the templates and a KIT record of the release under
  `/usr/share/proved-port/`. The port's Makefile names the same release
  and includes the installed targets:

  ```make
  PROVED_PORT_VERSION = X.Y.Z
  PROVED_PORT_HOME ?= /usr/share/proved-port
  include $(PROVED_PORT_HOME)/make/proved-port.mk
  ```

  The kit's targets then fail when the installed kit or counter is
  another release. `kit/count/README.md`, "Adopting: the package", has
  the details, including how to build the counter on a host without the
  package.
- **Templates.** A new port copies `kit/templates/`: the Verus ledgers
  (`verus/FLOOR`, `RLIMIT`, `REACH`, `MINTS`, `count.toml`), the check
  files, the reviewer configuration, the Claude Code settings and the
  document skeletons, with the placeholders filled. Each of the kit's
  Verus files (`kit/make/VERUS-FILES`) but
  `.chainguard/review-prompt.md` keeps the kit's header, with no
  `# kit:` line: the release is the package pin. The other templates are
  starting points. `make verus-headers` holds the kit's Verus files to
  the installed templates
  ([kit/templates/README.md](kit/templates/README.md)).
- **Changes.** A port carries no copy of the kit. A change to the
  counter or the make targets lands here and ships in a release; a new
  release reaches a port as one pull request that moves the package
  pins, `PROVED_PORT_VERSION` and the proved-port marketplace ref in
  `.claude/settings.json` together, and takes any new text in the held
  headers (the kit entry of `kit/templates/.chainguard/deps.yaml`).

A port that still carries vendored `verus/count/` and `verus/make/`
copies is on kit 0.2.4, and moves to the package in one pull request
(`kit/count/README.md`, "Moving from a vendored copy").

## What "proved" means here

A port built this way makes precise claims, and so should anyone
describing it:

- Verified code is checked by Verus against stated specifications.
  Coverage is counted and ratcheted (`FLOOR`); nothing new is trusted
  without a seam record saying why.
- Trusted code (the C ABI boundary, the operating system, a machine
  model) is listed, justified and, where Kani can check it, discharged
  on bounded inputs.
- Agreement with the C library is shown by differential and fuzz
  comparison, with every known difference numbered in a ledger.

So a claim reads "proved, modulo these seams", never "proved safe"
without its scope.

## License

Apache License 2.0; see [LICENSE](LICENSE).
