---
slug: draft
shortname: PMH-0001-HARNESS-SPEC
title: workingdoge.com/premath/PMH-0001-HARNESS-SPEC
name: Premath Meta-Harness HarnessSpec
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - harness
  - spec
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`HarnessSpec` types a harness specimen before it is evaluated, compared,
retired, or mutated. It names the specimen's shape and authority references; it
does not inline the whole runtime, sandbox, memory system, verifier, or
orchestration graph.

## 2. Minimal shape

```text
HarnessSpec :=
  Sigma {
    id                   : CID
    version              : SemVer
    name                 : String
    task_domain_ref      : CID
    model_profile_refs   : List CID
    interface_profile_ref: CID
    context_policy_ref   : CID
    tool_surface_ref     : CID
    authority_policy_ref : CID
    trace_contract_ref   : CID
    source_ref           : CID
    source_digest        : SHA256
    origin               : HarnessOrigin
    created_at           : Timestamp
  }
```

`source_ref` identifies the candidate realizer. `source_digest` binds the bytes
that were admitted. A candidate whose source bytes change without a new
`HarnessSpec` is a different specimen.

## 3. Origin

```text
HarnessOrigin :=
  Sigma {
    kind      : manual | generated | imported
    actor_ref : Optional CID
  }
```

The origin records provenance. It does not grant authority.

## 4. Non-goals

`HarnessSpec` MUST NOT embed:

- sandbox internals,
- full verifier code,
- a giant memory implementation,
- a concrete eval suite,
- provider secrets,
- authority-state write capabilities.

Those are bound by `RuntimeClosure`, `EvalPolicy`, and kernel admission.

## 5. Candidate judgment

A typed candidate has judgment form:

```text
Gamma |- H : HarnessSpec
```

where `Gamma` is witnessed context and `H` is the candidate specimen. Source
files, prompts, scripts, and templates are realizers of `H`; they are not the
typed candidate by themselves.
