# Toy vector tooling (adapter-only)

This directory contains small Python scripts that generate and run the **toy semantic vector suite**
defined in:

- `specs/premath/raw/BASEAPI-TOY-VIEWS.md`
- `specs/premath/raw/TOY-VECTORS.md`

Gate decisions are not implemented here. The adapter calls
`premath toy-gate-check`, which runs the Rust `premath-kernel` checker.

## Usage

From the repository root:

```bash
python3 tools/toy/gen_toy_vectors.py --out tests/toy/fixtures
python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
cargo test -p premath-kernel --test toy_vectors
```

The runner compares only stable fields:

- `result`
- each failure's `class`, `lawRef`, and `witnessId`

It intentionally ignores wording differences in `message`.
