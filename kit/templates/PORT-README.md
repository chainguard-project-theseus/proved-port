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

`make verify` runs every target the Makefile's `VERIFY_TARGETS` lists
(the kit's default is `verus kani`): everything the checks verify, as
one command. The port's package build runs it too, with
`VERUS_BASE=none`, so a target a check adds to the port's Verus or Kani
verification goes into `VERIFY_TARGETS` in the same pull request.

The package build takes its defaults: the latest kit, Verus and Kani,
and every vCPU. It does not pass `PROVED_PORT_VERSION`, so the
Makefile's pin is still held to the installed kit: once a kit release
is tagged and packaged, the package build fails at that check until the
port's pin moves to it (the deps agent's kit entry). A Verus release
the port's pins lag fails it too: `cargo verus` refuses a vstd pin in
Cargo.toml that is not the installed Verus's. The package build also
holds the `kani~` and `cbmc~` pins in the port's checks to the Kani and
CBMC it installs; nothing in the kit does that (`make kani` checks
Kani's version only when the port sets `KANI_VERSION`). To build
meanwhile, the package pins the older release, with a comment saying
why and until when (the issue that moves the port's pin); any other
departure from the package build's defaults carries one too.

## Contributing

CONTRIBUTING.md has the rules every change follows, and AGENTS.md the
workflow agents follow.
