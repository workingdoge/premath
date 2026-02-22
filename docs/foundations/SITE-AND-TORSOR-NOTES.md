# Site and Torsor Notes

These notes are non-normative.

## Why this is useful

Premath already has the key semantics:

- context change (reindexing),
- admissible covers,
- contractible gluing.

The site/sheaf/stack/torsor language gives a compact mathematical rendering of
the same structure, which helps us reason about coherence and extensions
without adding a second architecture.

## Minimal translation table

- Premath context world -> base category `Ctx`.
- Admissible cover -> topology/coverage `J`.
- Definables-in-context -> presheaf/sheaf/stack object.
- Transport coherence -> naturality.
- Glue or reject witness -> descent or obstruction.
- Twist/extension class -> torsor/`Ext` view over base data.

## Design guardrail

Do not rebuild Premath in parallel.

- Keep one acceptance authority: checker + deterministic witnesses.
- Keep site/torsor language as semantic compression over existing pipelines.
- Add new encodings only when they reduce net complexity.

## Practical reading

For current repo work, this is primarily useful for:

- shaping coherence obligations beyond transport functoriality,
- specifying glue-or-witness surfaces for richer evidence types,
- expressing non-canonical split behavior (extension classes) without leaking
  policy authority into proposal layers.

## Current grounding in the codebase

The abstract site language is already grounded in deterministic checker
obligations and vectors:

- `transport_functoriality`: identity/composition/naturality transport laws.
- `coverage_base_change`: pullback stability of admissible covers.
- `coverage_transitivity`: closure of cover-of-cover composition.
- `glue_or_witness_contractibility`: deterministic "exactly one of glue or
  obstruction" descent shape.

These are executed through `premath coherence-check` using
`specs/premath/draft/COHERENCE-CONTRACT.json` and fixtures under
`tests/conformance/fixtures/coherence-site`.

This keeps the foundation principle intact: minimum encoding, maximum
expressiveness. Site/torsor terminology compresses reasoning while acceptance
authority remains the existing checker + witness pipeline.
