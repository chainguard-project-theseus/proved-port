# {PORT}

[![Verus: 0.0% of functions, 0.0% of lines](https://img.shields.io/badge/Verus-0.0%25%20of%20functions%2C%200.0%25%20of%20lines-blue)](verus/README.md#coverage)

{PORT} is a memory-safe Rust port of {LIBRARY} {UPSTREAM_VERSION},
a drop-in replacement for its C ABI.

{What the port covers, what it does not yet, and where it ships.}

## Verification

The badge above gives verus/FLOOR's `functions` and `lines` figures,
the shares of the shipped crates' executable functions and of their
lines that Verus checks (verus/README.md, "Coverage"). The verus check
fails when the badge disagrees with verus/FLOOR, so the pull request
that raises FLOOR, which does nothing else but in the port's first
proof pull request (verus/FLOOR's header),
replaces the line with the one `verus-count --badge .` (the kit
counter, from the theseus-proved-port package) prints.

## Building and testing

{The make targets, and the toolchain each needs.}

The Verus targets need the proved-port kit at the release the Makefile's
`PROVED_PORT_VERSION` names. The Checks environments and
`.chainguard/ci/env/dev.yaml` install it as the Wolfi package
`theseus-proved-port`. On a host without the package, such as a Mac,
install the counter at that release with `cargo install --locked --git
<proved-port's URL> --tag vX.Y.Z verus-count` (or `cargo install --locked --path kit/count` from
a proved-port clone at the tag), point `PROVED_PORT_HOME` at an install
root laid out as the package's `/usr/share/proved-port` (`make/`,
`templates/` and `KIT`), and pass `VERUS_COUNT=/path/to/verus-count`
when the counter is not on PATH.

## Contributing

CONTRIBUTING.md has the rules every change follows, and AGENTS.md the
workflow agents follow.
