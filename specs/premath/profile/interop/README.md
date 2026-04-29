# Premath Interop Profile Map

This directory is the profile map for deterministic interchange artifacts.

Current authority files remain in `../../draft/` for path stability:

- `KCIR-CORE.md`
- `NF.md`
- `NORMALIZER.md`
- `REF-BINDING.md`
- `WIRE-FORMATS.md`
- `ERROR-CODES.md`

Profile-local orchestration lives here:

- `BIDIR-DESCENT.md` — full-profile bidirectional verifier orchestration over
  Core obligation/discharge.

Profile rule:

- Interop is optional and claim-scoped.
- Interop may constrain artifact form, normal forms, references, and wire
  behavior.
- Interop must not create a second admissibility law; accepted/rejected meaning
  still factors through `PREMATH-KERNEL`, `OBLIGATION-DISCHARGE`, `GATE`, and
  `WITNESS-ID`.

Any further physical moves into this directory should include updated
traceability, doctrine-site references, conformance fixtures, and decision-log
evidence.
