# Toy semantic vectors

Fixtures live in `tests/toy/fixtures/` and are specified by:

- `specs/premath/raw/BASEAPI-TOY-VIEWS.md`
- `specs/premath/raw/TOY-VECTORS.md`

To regenerate fixtures and run the Rust-native toy Gate check:

```bash
python3 tools/toy/gen_toy_vectors.py --out tests/toy/fixtures
cargo test -p premath-kernel --test toy_vectors
```
