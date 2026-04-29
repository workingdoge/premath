# Absorbed Raw Spec Pack

This directory holds raw material that was absorbed into the repository but is
not part of Premath Core.

Current tracks:

- ASP/identity/JWT raw pack: `ASP-IDENT-OVERVIEW.md`, `asp/ASP0.md`, and
  `ident/*.md`.
- Cover terminality raw notes: `COVER-TERMINALITY*.md`.
- Original bundle metadata: `README-PREMATH-SPECS.md` and
  `premath-specs.toml`.

Authority rule:

- These files are non-promoted.
- Normative language inside these files is proposal language unless a future
  promoted profile explicitly claims it.
- Do not wire JWT/JWKS runtime search, refresh, or cache behavior into Premath
  Core.

If this material graduates, it should move either to
`specs/premath/profile/identity/` or to a separate identity/auth site with a
Premath checker boundary.
