---
slug: draft
shortname: PMH-0002-RUNTIME-CLOSURE
title: workingdoge.com/premath/PMH-0002-RUNTIME-CLOSURE
name: Premath Meta-Harness RuntimeClosure
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - runtime
  - closure
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`RuntimeClosure` binds the carrier conditions under which a harness specimen is
realized. It is the runtime part of:

```text
Candidate x RuntimeClosure x EvalPolicy -> EvalReceipt | Obstruction
```

## 2. Shape

```text
RuntimeClosure :=
  Sigma {
    id                 : CID
    kind               : local_process | docker | nix | hosted_sandbox
    model_profile_ref  : CID
    runner_digest      : SHA256
    dependency_lock_ref: Optional CID
    environment        : RuntimeEnvironment
    determinism        : DeterminismProfile
    created_at         : Timestamp
  }
```

```text
RuntimeEnvironment :=
  Sigma {
    network    : disabled | restricted | enabled
    shell      : disabled | mediated | enabled
    filesystem : archive_api_only | sandboxed | host
    secrets    : none | mounted | brokered
  }
```

```text
DeterminismProfile :=
  Sigma {
    seed             : Optional Integer
    temperature      : Optional Number
    retry_policy_ref : Optional CID
  }
```

## 3. V0 profile

The v0 PMH profile fixes:

```text
network    = disabled
shell      = disabled
filesystem = archive_api_only
secrets    = none
```

The v0 profile exists to test the kernel boundary before introducing shell,
sandbox, provider, or secret complexity.

## 4. Replay binding

Every `EvalReceipt` MUST reference the `RuntimeClosure` used for the run. If a
later replay attempt cannot reconstruct the closure, the system MUST emit an
`UnreplayableReceipt` rather than silently dropping replayability.
