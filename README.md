# INF

This crate implements a low-level parser for the Setup Information (INF) file
format. INF is a text format organized into named sections, and is mainly used
by driver packages on Windows to store installation information.

Syntax validation is based on [Microsoft's INF specification][spec]. This crate
aims to provide detailed, helpful error messages when syntax rules are
violated.

This crate is an independently testable part of a larger project, [ani2xcur].

The project was split into separate repositories for two reasons: first, it
makes it easier to reuse this library in other projects; and second, it allows
independent components to run its own checks in localized CI workflows (which
is something I am currently experimenting with).

## Usage

Use cargo to add this crate to your project:

```bash
cargo add --git https://github.com/nicdgonzalez/inf.git
```

Or add the following line to your `Cargo.toml`:

```toml
[dependencies]
inf = { version = "0.1.0", git = "https://github.com/nicdgonzalez/inf.git" }
```

[ani2xcur]: https://github.com/nicdgonzalez/ani2xcur
[spec]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/general-syntax-rules-for-inf-files
