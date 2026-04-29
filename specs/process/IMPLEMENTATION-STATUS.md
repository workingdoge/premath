# Premath Implementation Status Snapshot

Status: non-normative process snapshot

This file preserves phase notes that were removed from
`specs/premath/draft/SPEC-INDEX.md` during the scope-diet cleanup.

Current live status authority is issue memory:

- `.premath/issues.jsonl`
- `premath issue list`
- `premath issue ready`
- `premath issue blocked`

Do not treat this file as current project authority if it conflicts with issue
memory.

## Snapshot From 2026-04-24

Current phase at the time of the snapshot:

- KCIR self-hosting phase 3 was active under `bd-287`.
- The statement-ID/KCIR projection indexing follow-on was not active until a
  corresponding issue existed in issue memory.

Active epic IDs:

- `bd-287`: KCIR self-hosting phase 3.

Recently closed epic IDs:

- `bd-262`: KCIR self-hosting phase 2.

Phase-3 dependency spine at the time of the snapshot:

- `bd-288`: architecture contract (target-state vs transition-state; closed).
- `bd-289`: spec/index glue (closed).
- `bd-234`: host-action mapping contract/checker binding (closed).
- `bd-290`: control-plane parity (closed).
- `bd-235`: local REPL lease-op parity boundary (closed).
- `bd-291`: implementation (open; next).
- `bd-292`: conformance (open; blocked on `bd-291`).
- `bd-293`: docs/traceability closure (open; blocked on `bd-292`).

Sidecar issue:

- `bd-294`: docs topology/navigation refactor (closed; authority entrypoints +
  design lane folders; not a semantic phase-3 dependency).

Active non-epic blocker at the time of the snapshot:

- `bd-67` (`blocked`, manual): governance reviewer-pool readiness.
