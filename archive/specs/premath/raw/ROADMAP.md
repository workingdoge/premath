---
slug: raw
shortname: ROADMAP
title: workingdoge.com/premath/ROADMAP
name: Roadmap (Informative)
status: raw
category: Informational
tags:
  - premath
  - kernel
  - roadmap
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This document is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

This document is **informative** and intentionally narrow.

It records a compact phase model for orientation, but it is **not the
authoritative source of active work**.

Authoritative planning surfaces are:

- `.premath/issues.jsonl` (query/mutate through `premath issue ...` surfaces)
- `specs/process/decision-log.md` (binding architecture/process decisions)

If this file conflicts with those surfaces, treat this file as stale and follow
the issue graph + decision log.

## 2. Current status

As of 2026-02-22:

- Core kernel + interop contract surfaces have been promoted to `draft`.
- Coherence and instruction/proposal checking surfaces are executable and
  merge-gated.
- Forward planning is tracked as issue graph epics/tasks (`bd-*`) with
  dependency edges.

## 3. Phase model (historical heuristic)

### Phase A — Raw publication

- Publish raw specs for the kernel bundle.
- Publish raw operational companions (`raw/TUSK-CORE`, `raw/SQUEAK-CORE`, `raw/CI-TOPOS`) without making them normative for kernel claims.
- Start a decision log.
- Accept breaking changes freely.

### Phase B — Running code

- Implement at least one verifier and one producer.
- Build golden + adversarial vectors (including Gate vectors).
- Prefer settling disputes by tests.

### Phase C — Draft 1 (first contract)

- Copy raw specs to numbered drafts.
- Freeze witness formats and conformance taxonomy.

### Phase D — Stable

- Promote to stable once used by third parties.
- Restrict changes to errata and clarifications.

## 4. Planning policy

Use this file for high-level orientation only.

For execution ordering, ownership, blocking analysis, and acceptance evidence:

1. use `.premath/issues.jsonl` as the canonical task memory substrate;
2. use `specs/process/decision-log.md` as the canonical architectural intent log;
3. require deterministic witnesses/checks for any promoted contract changes.
