# INF

A read-only parser for the INF file format.

## Getting started

Use cargo to add the library to your project:

```bash
cargo add --git https://github.com/nicdgonzalez/inf.git
```

Or, add the following to your `Cargo.toml`:

```toml
[dependencies]
inf = { version = "0.1.0", git = "https://github.com/nicdgonzalez/inf.git" }
```

### Basic example

A basic example that iterates over all sections and entries in an INF file.

```rust
use std::fs;
use inf::Inf;

let mut reader = fs::File::open("Install.inf")?;
let inf = Inf::from_reader(&mut reader)?;

for section in inf.sections() {
    println!("{}", section.name());

    for entry in section.entries() {
        println!("{entry:?}");
    }
}
```
