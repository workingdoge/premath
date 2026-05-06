# Premath Interop Profile Map

This directory is the profile map for deterministic interchange artifacts.

KCIR carrier substrate authority now lives in `fish/sites/kcir`. This profile
is the Premath-side adapter that checks whether KCIR-carried artifacts satisfy a
Premath claim boundary.

Current Premath adapter files remain in `../../draft/` for path stability and
existing checker tooling:

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
- KCIR substrate meaning belongs to `fish/sites/kcir`, not to Premath Core.
- Interop may constrain artifact form, normal forms, references, and wire
  behavior.
- Interop must not create a second admissibility law; accepted/rejected meaning
  still factors through `PREMATH-KERNEL`, `OBLIGATION-DISCHARGE`, `GATE`, and
  `WITNESS-ID`.

See `../kcir/README.md` for the explicit Premath/KCIR/Kurma split.

Any further physical moves into this directory should include updated
traceability, doctrine-site references, checker fixtures, and decision-log
evidence.
