# INF Decoder

A decoder for the INF file format.

## Installation

Install from GitHub using uv:

```bash
uv add git+https://github.com/nicdgonzalez/inf.git
```

## Usage

Import `inf` and use the `load` function:

```python
import inf

with open("Install.inf", "r") as f:
    data = inf.load(f.read())

print(data)
```

## Limitations

I only implemented what I needed for my use case without referencing the formal
specification. The parsing is nowhere near accurate yet. Proceed with caution!
