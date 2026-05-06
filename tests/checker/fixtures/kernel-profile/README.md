# Kernel Profile Fixtures

Executable cross-model kernel profile vectors compare stable Gate outcomes for
shared semantic scenarios across:

- semantic toy fixtures (`tests/toy/fixtures`)
- KCIR toy fixtures (`tests/kcir_toy/fixtures`)

Run with:

```bash
cargo test -p premath-kernel --test toy_vectors
python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures
```
