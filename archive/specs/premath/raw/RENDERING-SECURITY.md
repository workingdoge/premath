---
slug: raw
shortname: RENDERING-SECURITY
title: workingdoge.com/premath/RENDERING-SECURITY
name: Spec Rendering Security Policy
status: raw
category: Standards Track
tags:
  - premath
  - security
  - rendering
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
specification are to be interpreted as described in RFC 2119 (and RFC 8174 for
capitalization).

## 1. Scope

Spec markdown MUST be treated as untrusted input at render time.

Implementations that render Premath specs to HTML MUST follow this policy.

## 2. Rendering pipeline (normative)

Implementations MUST use a pipeline that:

- parses markdown to an AST,
- converts markdown AST to HTML AST with raw HTML disabled,
- sanitizes the HTML AST using an explicit allowlist,
- stringifies the sanitized HTML AST.

## 3. Allowed HTML (recommended baseline allowlist)

Allowed elements:

`a`, `blockquote`, `br`, `code`, `em`, `h1`-`h6`, `hr`, `li`, `ol`, `p`, `pre`,
`strong`, `table`, `thead`, `tbody`, `tr`, `th`, `td`, `ul`.

Allowed attributes:

`a[href,title]`, `th[align]`, `td[align]` (align limited to `left|center|right`).

## 4. URL and link rules

Allowed link schemes: `https`, `http`, `mailto`.

In-page anchors (`#...`) and relative links (`/`, `./`, `../`) are allowed.

Schemes `javascript:`, `data:`, `vbscript:`, and `file:` MUST be rejected.

For external links (`http:`/`https:`), renderers SHOULD add:

- `rel="noopener noreferrer"`
- `referrerpolicy="no-referrer"`
