---
slug: process
shortname: COSS-PROCESS
title: workingdoge.com/premath/COSS-PROCESS
name: COSS Process
status: draft
category: Best Current Practice
tags:
  - coss
  - process
editor: arj <arj@workingdoge.com>
contributors: []
---

## Status

This is a project-local summary of how we apply the **Consensus-Oriented
Specification System (COSS)** to the Premath kernel spec set. It is informative
but treated as the governing process document for this repository.

## License

This document (and the surrounding spec set, unless noted otherwise) is
dedicated to the public domain under **CC0 1.0** (see `../../LICENSE`).

## Change process

The spec set is maintained using a lightweight editorial process that aims for:

- **rough consensus** (not voting), and
- **running code** (conformance tests and interoperable implementations).

Each specification MUST have a single responsible **editor**.

### Lifecycle

Each specification progresses through:

1. **raw** — experimentation, no contractual weight
2. **draft** — numbered; a contract between editor and implementers
3. **stable** — used by third parties; changes mostly errata/clarifications
4. **deprecated** — replaced by a newer draft
5. **retired** — historical

Only raw and draft specifications may be deleted.

### Branching and dispute resolution

If a technical dispute cannot be resolved, any member MAY branch by copying a
spec into a new numbered version (or new raw name) and becoming the editor of
that branch. Community adoption determines which branch becomes dominant.

### “Running code” rule

Changes that affect interoperability MUST come with:

- test vectors (golden and/or adversarial), and/or
- reference implementation updates.

### Editorial decisions and decision log

Significant decisions SHOULD be recorded in `decision-log.md`, including:

- what decision was made
- context and alternatives
- rationale
- links to issues/PRs

## Language

Normative text uses the key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**,
**SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and
**OPTIONAL** as described in RFC 2119 (and RFC 8174 for capitalization).
