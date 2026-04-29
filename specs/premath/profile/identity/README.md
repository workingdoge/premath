# Premath Identity/Auth Holding Map

Identity/auth material is not Premath Core.

The absorbed ASP/IDENT/JWT pack currently lives under repository-level
`specs/raw/`:

- `specs/raw/ASP-IDENT-OVERVIEW.md`
- `specs/raw/asp/ASP0.md`
- `specs/raw/ident/IDENT0.md`
- `specs/raw/ident/JWT0.md`
- `specs/raw/ident/JWT0-OBS.md`
- `specs/raw/ident/JWT0-SEARCH.md`
- `specs/raw/ident/JWT1-JWKS0.md`
- `specs/raw/COVER-TERMINALITY*.md`

Holding rule:

- Keep this material raw until identity/auth is explicitly accepted as a
  Premath profile or moved to a separate site.
- JWT/JWKS runtime search, refresh, and cache behavior must not become Premath
  Core by adjacency.
- Any promotion needs a profile boundary, executable vectors, traceability, and
  a decision-log entry.
