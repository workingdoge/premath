# Gate Fixtures

Executable deterministic gate vectors for:

- stability (`GATE-3.1`)
- locality (`GATE-3.2`)
- descent existence (`GATE-3.3`)
- contractible glue uniqueness (`GATE-3.4`)

Run with:

```bash
cargo test -p premath-kernel
python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
```
