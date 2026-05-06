# Topology V2

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Topology V2 reshapes placement decisions around architectural shape before repo
ownership. Repositories are hosts for layers; they are not the ontology.

This note is now a local companion to `ATLAS.md`. Atlas names the site-of-sites
model that owns cross-repo placement doctrine. This file keeps the shape-layer
routing rules visible from Premath while Premath is reduced back toward
checker/kernel ownership.

This replaces the first question:

```text
which repo owns this?
```

with:

```text
what shape layer carries authority here?
```

## 2. Shape Layers

| Layer | Role | Authority question |
| --- | --- | --- |
| substrate | primitive geometry/state language | what kinds of local state and overlap exist? |
| law | admissible claims, transitions, invariants, and failure classes | what is valid? |
| carriage | reusable executable realization, replay, normalization, and verification | how is validity computed generically? |
| instrument | operator/runtime tool surface | how does an operator or agent act on stable authority? |
| projection | human/product/query view over authority state | how is authority observed without becoming authority? |
| proof | live domain instance under real constraints | where is the shape demonstrated? |

The routing order is:

1. classify the shape layer,
2. name the authority-bearing object,
3. name projection-only objects,
4. declare the simplex edge between layers,
5. select the repo host,
6. shape one lane wire.

## 3. Host Map

The current host map is secondary to the shape layer:

| Host | Default layer |
| --- | --- |
| candidate `simplex`; provisionally `fish/sites/nerve` | substrate language for simplex, boundary, incidence, patch, and local-to-global coherence |
| `fish/sites/premath` | checker/kernel profile for admissibility decisions, witnesses, failures, and checker claims |
| `fish/sites/kcir` | generic carrier-substrate vocabulary for durable artifacts, refs, rows, nodes, dependencies, normal forms, witness records, failure reports, and carrier profiles |
| `kurma` | reusable carriage once a method becomes generally executable beyond one tool |
| `tusk` | instrument layer for operator workflow, MCP/CLI/daemon surfaces, worker loops, repo binding, and compatibility adapters |
| downstream repos | projection and proof under local product or operator policy |

The host map may change, but the shape-layer distinction must not collapse.

## 4. Authority And Projection

Authority-bearing objects are the objects whose canonical form decides
validity. Projection objects are derived views.

Examples:

| Authority object | Projection object |
| --- | --- |
| `WorkSimplex` | issue row |
| `WorkStepNF` | command transcript |
| `WorkPatchWitness` | ready/blocked board |
| mutation admissibility decision | dashboard status |
| canonical replay projection | dependency graph |

Projection objects may be operationally important, but they do not authorize
state transitions.

## 5. Tracker Worked Example

The tracker should not be modeled as a graph with stronger checks. It should be
modeled as simplex-native work state with graph views derived from it.

Tracker shape:

- `WorkSimplex`: a local work-state carrier.
- `WorkStepNF`: the canonical claim for one transition such as claim, update,
  discover, close, lease-renew, or lease-release.
- `WorkPatchWitness`: a finite active work patch with enough evidence that its
  boundary states compose.
- `ready`, `blocked`, `dependency graph`, `epic`, `queue`, and `sprint`: derived
  projections.
- `bd`: compatibility/import/export substrate during transition, not the
  conceptual parent.

Routing:

| Slice | Shape layer | Host |
| --- | --- | --- |
| simplex and patch vocabulary | substrate | candidate `simplex`; provisionally `fish/sites/nerve` |
| work-state meaning | law/semantics | candidate `work`; not Premath |
| tracker admissibility checker | checker | `fish/sites/premath` raw `WORK-TRACKER-CHECKER-PROFILE` |
| reusable replay/checker implementation | carriage | `kurma` when generalized |
| CLI/MCP/daemon/operator tool | instrument | `tusk` |
| ready/blocked/graph/board views | projection | `tusk` or downstream views |
| one real workflow proof | proof | first live consumer repo |

## 6. Placement Rules

Use these before making a repo decision:

1. If the question is "what object exists?", route to substrate and law.
2. If the question is "what transition is admissible?", route to law.
3. If the question is "how is it replayed, normalized, or verified
   generically?", route to carriage.
4. If the question is "how does an operator use it?", route to instrument.
5. If the question is "how do we see it?", route to projection.
6. If the question is "does this work in a real domain?", route to proof.

Do not route by overloaded nouns such as `tracker`, `memory`, `runtime`,
`backend`, or `knowledge`. Decompose the noun into shape layers first.

## 7. KCIR Worked Example

KCIR should be modeled as a carrier-substrate site, not as a Premath submodule
or a Kurma runtime package.

KCIR shape:

- `KCIRArtifact`: durable carried object after a source site has authored
  meaning.
- `KCIRRef`: stable artifact reference.
- `KCIRNode`: typed payload member.
- `KCIRDependency`: carried edge vocabulary.
- `KCIRNormalForm`: normalized record over a carried artifact or node.
- `KCIRWitnessRecord`: evidence record associated with a carried target.
- `KCIRFailureReport`: preserved negative/obstructed evidence.

Routing:

| Slice | Shape layer | Host |
| --- | --- | --- |
| Premath admissibility law and Gate verdict | law/checker | `fish/sites/premath` |
| Generic artifact/ref/node/dependency vocabulary | substrate | `fish/sites/kcir` |
| Executable lowering, stores, codecs, normalizers, receipts | carriage | `kurma` |
| Operator workflow around stable refs | instrument/projection | `tusk` |
| Theory-specific `ObjNF`/`MorNF` meaning | law/substrate | source theory site, for example `fish/sites/nerve` |

## 8. Compatibility Rule

Compatibility surfaces are projections or adapters unless they explicitly carry
the new authority object.

For the current tracker transition:

- `bd` remains useful for bootstrap operations, imports, and comparison.
- `.beads` and `.premath` drift is operational evidence that the authority
  object is underspecified.
- A future simplex-native tracker should define its authority in Premath and
  expose operator tooling through Tusk.
- Graph exports should be treated as derived compatibility views.

## 9. Lane Wire Template

Every consequential lane should declare:

```text
shape layer:
authority object:
projection objects:
simplex edge:
repo host:
input context:
output artifact:
verification boundary:
landing boundary:
```

This keeps repo placement subordinate to the architectural shape.

## 10. Verification

This note is non-normative. It should remain coherent with:

- `docs/design/control-plane/ATLAS.md`
- `fish/sites/kcir/specs/draft/KCIR-0001-SUBSTRATE.md`
- `fish/sites/atlas/specs/raw/ATLAS-0002-WORK-TRACKER-COVER.md`
- `fish/sites/atlas/specs/raw/ATLAS-0003-SIMPLEX-FACTOR-CANDIDATE.md`
- `specs/premath/raw/WORK-TRACKER-CHECKER-PROFILE.md`
- `docs/design/control-plane/MEMORY-LANES-CONTRACT.md`
- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`

Future normative work should promote only the law-bearing portions into
Premath specs after the simplex-native tracker profile is explicit.
