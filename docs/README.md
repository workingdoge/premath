# Docs Index

This directory contains non-normative documentation and implementation notes.

- `design/README.md` — lane-grouped design notes.
- `design/runtime/README.md` — Tusk/Premath runtime design notes.
- `design/transport/README.md` — Squeak/SigPi transport and placement notes.
- `design/control-plane/README.md` — CI, issue memory, doctrine routing, and
  operator control notes.
- `design/control-plane/CI-PROVIDER-BINDINGS.md` — provider-specific CI check
  bindings (GitHub example; canonical gate contract remains provider-agnostic).
- `design/control-plane/ARCHITECTURE-MAP.md` — one-page doctrine-to-operation
  architecture map.
- `design/GLOSSARY.md` — shared terminology used across design docs.
- `foundations/` — explanatory background notes for kernel and SigPi concepts.
- `observation/index.html` — lightweight local dashboard for Observation Surface v0.
- `../specs/README.md` — spec authority entrypoint.
- `../specs/premath/draft/SPEC-INDEX.md` — normative spec entrypoint and
  claim/profile map.

Boundary rule:

- `docs/` is explanatory only.
- Normative authority is always under `specs/`.

For process/governance, see `../specs/process/README.md`.

Local dashboard quickstart:

```bash
sh tools/ci/run_task.sh ci-observation-build
sh tools/ci/run_task.sh ci-observation-serve
python3 -m http.server 43173 --directory docs
```

Then open `http://127.0.0.1:43173/observation/`.

Or start both docs preview + observation API together:

```bash
sh tools/ci/run_task.sh pf-start
```
