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
marketplace's GitHub source.

## How a port consumes the kit

A port vendors the kit rather than depending on it, since a port's CI
cannot reach this repository:

- **Vendoring.** `kit/sync.sh PORT_ROOT`, run from a clone of this
  repository at the commit to take (a release tag where there is one),
  copies `kit/count` (the coverage counter) into the port's
  `verus/count/` and `kit/make` (the make targets and `kani.sh`) into
  its `verus/make/`. It records the commit in `verus/count/KIT`, with
  its tag, or `tag none` for a copy taken between releases.
  `kit/sync.sh --check PORT_ROOT` fails unless both copies match that
  commit byte for byte. Whoever changes a copy runs it, since the
  port's CI cannot.
- **Templates.** A new port copies `kit/templates/`: the Verus ledgers
  (`verus/FLOOR`, `RLIMIT`, `REACH`, `MINTS`, `count.toml`), the check
  files, the reviewer configuration and the document skeletons, with
  the placeholders filled. Each of the kit's Verus files
  (`kit/make/VERUS-FILES`) but `.chainguard/review-prompt.md` keeps the
  kit's header and its `# kit: proved-port vX.Y.Z (<sha>)` line; the
  other templates are starting points that carry no kit line.
  `make verus-headers` holds the kit's Verus files to the templates of
  the commit KIT records ([kit/templates/README.md](kit/templates/README.md)).
- **Changes.** A port never edits its vendored copies. A change to the
  counter or the make targets lands here first; the port then re-syncs
  to a commit that has it, and moves its kit lines to that commit.

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
