# CI Provider Adapters

Provider adapters map provider-specific CI environment into the canonical
Premath CI ref contract:

- `PREMATH_CI_BASE_REF`
- `PREMATH_CI_HEAD_REF`

Current adapter:

- `export_github_env.py` maps `GITHUB_BASE_REF`/`GITHUB_SHA` to `PREMATH_CI_*`
  assignments.

Example:

```bash
python3 tools/ci/providers/export_github_env.py >> "$GITHUB_ENV"
```
