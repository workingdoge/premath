# Work Tracker Checker Fixtures

Executable deterministic vectors for raw `WORK-TRACKER-CHECKER-PROFILE` via:

- `premath work-tracker-check`

The suite checks the Premath checker endpoint only:

- canonical `WorkClaimNF` input shape,
- accepted/rejected checker decision projection,
- failure-class stability,
- projection-as-authority rejection,
- compatibility boundary for current tracker projections.

It does not define work semantics, tracker runtime, storage, CLI, MCP, daemon,
or UI behavior.

The old Python suite wrapper is retired. Keep this corpus as checker-boundary
input material until the work tracker lives in its owning site. Current native
surface:

```bash
cargo run --package premath-cli -- work-tracker-check --help
```
