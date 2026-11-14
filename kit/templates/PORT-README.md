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
`PROVED_PORT_VERSION` names. The Checks environments and the
development sessions' environment install it as the Wolfi package
`theseus-proved-port`. The Makefile includes the kit behind a guard
(the proved-port kit's kit/count/README.md, "Adopting: the package"),
so where the package is not installed the Verus targets fail saying so
and every other target still runs. On a host without the package, such as a Mac,
install the kit from a proved-port clone checked out at that release with
`sh kit/install.sh "$HOME/.local"`, then pass
`PROVED_PORT_HOME="$HOME/.local/share/proved-port"` and
`VERUS_COUNT="$HOME/.local/bin/verus-count"` to the Verus targets.

## Contributing

CONTRIBUTING.md has the rules every change follows, and AGENTS.md the
workflow agents follow.
