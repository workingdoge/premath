# Toy semantic vectors

Fixtures live in `tests/toy/fixtures/` and are specified by:

- `specs/premath/raw/BASEAPI-TOY-VIEWS.md`
- `specs/premath/raw/TOY-VECTORS.md`

To regenerate and run (non-normative tooling):

```bash
python3 tools/toy/gen_toy_vectors.py --out tests/toy/fixtures
python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
```
