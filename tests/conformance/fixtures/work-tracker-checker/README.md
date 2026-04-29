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

Run with:

```bash
python3 tools/conformance/run_work_tracker_checker_vectors.py
```
