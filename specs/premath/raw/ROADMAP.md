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

This document is **informative**. It outlines a suggested path from an initial
raw spec set to a draft/stable Premath kernel.

## 2. What to freeze first

In a consensus-oriented process, it is usually best to freeze the things that
are most expensive to change:

1. Reference model + binding interface (`draft/KCIR-CORE`, `draft/REF-BINDING`)
2. NF byte grammars (`draft/NF`)
3. Normalizer comparison key semantics (`draft/NORMALIZER`)
4. Bidirectional/descent obligations (`draft/BIDIR-DESCENT`)
5. Gate law wording + witness classes (`draft/GATE`)

Optional extensions (PullAtom, additional canonicalization modules, extra wire formats)
can be postponed until tests and multiple implementations justify freezing.

## 3. Suggested phases

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
