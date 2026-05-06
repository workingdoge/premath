# Premath Specs

This directory contains Premath's spec authority, grouped by lifecycle state.

## Entry Points

- `AUTHORITY-MAP.json` is the machine-readable classification of Premath Core,
  promoted profiles, raw holding areas, and adjacent-site boundaries.
- `draft/SPEC-INDEX.md` is the first authority entrypoint for current
  core/profile/raw/site boundaries.
- `draft/README.md` summarizes promoted draft contracts.
- `profile/README.md` summarizes optional overlays that become normative only
  when explicitly claimed.
- `raw/README.md` summarizes exploratory or informational specs that are not
  active checker-claims authority.

## Authority Split

- `draft/` is the active promoted contract surface. Not every draft file is
  Premath Core; some draft files are profile or control-plane contracts.
- `profile/` contains additive claim-scoped overlays.
- `raw/` is visible incubation material and historical context.

Implementation-facing design notes live under `../../docs/design/` and must
route back to this tree when they discuss normative behavior.
