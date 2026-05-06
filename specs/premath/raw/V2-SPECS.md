---
slug: raw
shortname: V2-SPECS
title: workingdoge.com/premath/V2-SPECS
name: KCIR Spec Bundle (Informative)
status: raw
category: Informational
tags:
  - premath
  - kernel
  - kcir
  - commitments
  - roadmap
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This document is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Scope

This document indexes the greenfield Premath kernel bundle.

## 2. Documents

- `draft/SPEC-INDEX`: front door (normative vs informative; checker claims)
- `draft/PREMATH-KERNEL`: definability kernel
- `draft/GATE`: admissibility laws + witness classes
- `draft/REF-BINDING`: backend-generic reference binding
- `draft/KCIR-CORE`: core KCIR model
- `draft/WIRE-FORMATS`: wire format registry
- `draft/NF`: normal form grammars
- `draft/NORMALIZER`: normalization and comparison keys
- `profile/interop/BIDIR-DESCENT`: bidirectional evaluation with descent obligations
- `raw/DSL`: dependency-pattern DSL (optional but recommended)
- `raw/OPCODES`: opcode contracts (minimal in this bundle)
- `draft/CHECKER-CLAIMS`: kernel checker profiles
- `draft/ERROR-CODES`: stable error codes
- `raw/RENDERING-SECURITY`: safe spec rendering policy

## 3. Operational companion docs (raw)

These documents describe an implementation-facing operational layer while
remaining outside the kernel checker core:

- `raw/SQUEAK-CORE`: inter-world transport/composition contracts
- `raw/SQUEAK-SITE`: runtime-location site contracts (`Loc`, covers, glue)
- checker projection discipline is carried by promoted checker contracts.
