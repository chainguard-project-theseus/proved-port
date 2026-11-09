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
  piece as `kit/PLAN.md` lays out; the ports it came from consume it.
  `kit/templates/` holds the document skeletons a new port copies.

The method is distilled from two ports in this organization:

- [wolfi-dev/pcre3](https://github.com/wolfi-dev/pcre3), a PCRE2 10.49
  replacement with a proved JIT on aarch64 and x86-64.
- [wolfi-dev/challoc](https://github.com/wolfi-dev/challoc), a
  `malloc` replacement.

## Install

Add the marketplace and install the plugin in Claude Code:

```
/plugin marketplace add wolfi-dev/proved-port
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
marketplace and plugin to that repository's `.claude/settings.json`.

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
