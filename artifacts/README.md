# Artifacts

Runtime-generated artifacts are written here.

Current artifact classes:

- `ciwitness/<instruction-id>.json`: CI instruction witness records emitted by
  `tools/ci/run_instruction.sh`.
- `observation/latest.json`: deterministic observation read model built from
  CI witness artifacts (`tools/ci/observation_surface.py build`).
- `observation/events.jsonl`: projection/event feed exported from the
  observation surface (suitable for downstream query/index adapters).
