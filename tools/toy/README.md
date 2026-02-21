# Toy vector tooling (non-normative)

This directory contains small Python scripts that generate and run the **toy semantic vector suite**
defined in:

- `specs/premath/raw/BASEAPI-TOY-VIEWS.md`
- `specs/premath/raw/TOY-VECTORS.md`

## Usage

From the repository root:

```bash
python3 tools/toy/gen_toy_vectors.py --out tests/toy/fixtures
python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
```

The runner compares only stable fields:

- `result`
- each failure's `class`, `lawRef`, and `witnessId`

It intentionally ignores wording differences in `message`.
