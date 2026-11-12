# {PORT}

[![Verus: 0.0% of functions, 0.0% of lines](https://img.shields.io/badge/Verus-0.0%25%20of%20functions%2C%200.0%25%20of%20lines-blue)](verus/README.md#coverage)

{PORT} is a memory-safe Rust port of {LIBRARY} {UPSTREAM_VERSION},
a drop-in replacement for its C ABI.

{What the port covers, what it does not yet, and where it ships.}

## Verification

The badge above gives verus/FLOOR's `functions` and `lines` figures,
the shares of the shipped crates' executable functions and of their
lines that Verus checks (verus/README.md, "Coverage"). The verus check
fails when the badge disagrees with verus/FLOOR, so a change that
raises FLOOR replaces the line with the one `verus-count --badge .`
(the counter verus/count builds) prints, in the same pull request.

## Building and testing

{The make targets, and the toolchain each needs.}

## Contributing

CONTRIBUTING.md has the rules every change follows, and AGENTS.md the
workflow agents follow.
