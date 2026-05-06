# Premath

**Bundle version:** `0.1.0` (parked)

Premath is a small checker doctrine for admissible definability.

A claim is Premath-admissible when it is stable under context change, local
over covers, and glues uniquely from compatible local data.

Premath Core decides admissibility. Profiles add structure. Downstream sites
run systems.

Premath does not own runtime orchestration, KCIR substrate semantics, agent
workflows, issue trackers, provider CI, or model execution. Those systems may
compile claims into Premath, but they are not Premath Core.

## System in 30 seconds

- **Semantic authority**: kernel + obligation/gate specs decide admissibility (`PREMATH-KERNEL`, `OBLIGATION-DISCHARGE`, `GATE`).
- **Profiles**: Interop, KCIR adapter, control-plane, and adjoints/sites
  profiles are optional and claim-scoped.
- **Control-plane consistency**: profile-local coherence checkers enforce
  spec/docs/contract parity and emit deterministic checker witnesses.
- **Runtime boundary**: host execution, hook management, retry/escalation, and provider artifact publication belong to Tusk or downstream operational sites.
- **Regression discipline**: native Rust tests, toy vectors, traceability, coherence, and drift-budget checks keep behavior stable as capabilities evolve.

## Layout

- `specs/premath/draft/` — promoted draft contracts (normative for active claims)
- `specs/premath/profile/` — optional claim-scoped overlays
- `specs/premath/raw/` — raw (experimental/informational) documents
- `specs/process/` — process docs (COSS lifecycle)
- `docs/foundations/` — explanatory foundations notes (non-normative)
- `docs/design/` — implementation-facing notes grouped by control-plane,
  transport, and operations lanes (non-normative)

## Start here

- `specs/premath/draft/SPEC-INDEX.md` — what is normative, what claims exist (Kernel vs Interop), and suggested reading orders.
- `specs/premath/draft/PREMATH-KERNEL.md` — definability kernel (contractible descent).
- `specs/premath/draft/OBLIGATION-DISCHARGE.md` — Core obligation records and deterministic discharge.
- `specs/premath/draft/GATE.md` — accepted/rejected Gate outcomes and failure classes.
- `specs/premath/draft/WITNESS-ID.md` — deterministic witness identity.
- `specs/premath/draft/CHECKER-CLAIMS.md` — Core and profile claim tokens.
- `specs/premath/draft/DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract.

## Checker Claims

Checker claims are profile-based. See:

- `specs/premath/draft/CHECKER-CLAIMS.md`
- `specs/premath/draft/CAPABILITY-VECTORS.md`

Interop documents (NF/normalizer/refs/wire/errors) are normative **only when their corresponding interop claims are asserted**.
Generic KCIR substrate authority is now factored into `fish/sites/kcir`.
Premath's interop docs are the Premath-side adapter that decides whether a
KCIR-carried artifact is acceptable for a Premath claim.

Continuation and tool-calling typestate belong to Tusk or a future continuation
site. Premath consumes compiled checker artifacts rather than owning that
normalizer.

## Toy suites

This repo includes two small, executable suites that exercise the **Gate laws**:

- **Semantic toy suite**: `premath toy-gate-check` + `tests/toy/fixtures/`
  - Fastest way to sanity-check stability/locality/descent through the Rust kernel.
  - Run: `cargo test -p premath-kernel --test toy_vectors`

- **KCIR toy suite**: `tools/kcir_toy/` + `tests/kcir_toy/fixtures/`
  - Compiles the semantic cases into **KCIR/NF-shaped fixtures**, then runs a
    minimal KCIR verifier + the Rust-native Gate check.
  - Compile: `python tools/kcir_toy/compile_kcir_toy_fixtures.py --in tests/toy/fixtures --out tests/kcir_toy/fixtures`
  - Run: `python tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures`

> Note: the `tools/kcir_toy` fixture generator uses a SHA-256 toy binder for reproducibility only.
> The normative kernel remains commitment-backend agnostic via `draft/REF-BINDING`.

Python tooling dependency convention:

- `requirements.txt` is the authoritative dependency list for `tools/` scripts.
- It is currently stdlib-only (intentionally empty), but any future third-party imports must be declared there.

## Dev Environment (Nix + Direct Commands)

This repo supports a Nix-first setup:

- `direnv` + `nix develop` provide system/native dependencies and shell tooling.
- Cargo and native Premath CLI commands are the repository check surface.

Tracked files:

- `flake.nix` (system layer)
- `.envrc` (repo-root-aware `use flake`)

Direnv setup:

```bash
direnv allow
```

Typical workflows:

```bash
# Nix-first lane (after direnv allow)
cargo test --workspace

# One-shot lane without entering the shell
direnv exec . cargo test --workspace

# Non-Nix lane
cargo test --workspace
```

The flake shell uses `devenv.root`, so raw `nix develop` needs the same root
override that `.envrc` supplies automatically. Prefer `direnv exec . <cmd>` for
one-shot commands outside an activated shell.

## Workspace layering

Runtime crates are split by responsibility:

- `crates/premath-kernel`:
  - Generic laws only (contexts, covers, reindexing, descent, witnesses).
  - No storage, tracker, workflow, or backend policy.
- `crates/premath-coherence`:
  - Typed profile/control-plane coherence evaluators used by checker commands.
  - Emits deterministic checker witness output over profile contracts.
- `crates/premath-cli`:
  - Composition point for checker and witness command surfaces.

This keeps Premath backend-generic. Tracker runtime, mutation workflow, and
issue-memory orchestration belong in Tusk or downstream operator surfaces.

Work-tracker boundary model:

- `premath work-tracker-check` is a profile/control-plane checker surface over
  normalized work-tracker claims. It is not part of `premath-kernel`.
- Premath does not own issue mutation, ready queues, dependency graph editing,
  or tracker storage.
- Tusk/downstream tracker sites own those runtime and workflow surfaces.

### Kernel vs KCIR note

Premath semantics and KCIR-style representation should stay decoupled:

- kernel semantics (`premath-kernel`) define laws and witness interfaces,
- KCIR is a separate carrier site for generic artifact substrate meaning,
- Premath's KCIR/interop profile checks acceptance of KCIR-carried artifacts,
- executable KCIR lowering, builders, stores, codecs, normalizers, and receipts
  belong in Kurma rather than Premath Core.

## Baseline gate

Run the local checker gate before commit:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures
cargo run --package premath-cli -- traceability-check
cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json
cargo run --package premath-cli -- drift-budget-check --json
cargo run --package premath-cli -- command-surface-check
cargo run --package premath-cli -- repo-hygiene-check
```

This enforces the current invariant gate:

- format check + clippy (`-D warnings`),
- build + Rust tests,
- KCIR toy vectors,
- coherence-contract obligation discharge validation,
- authority-map/traceability validation,
- drift-budget sentinel validation across docs/contracts/checkers/cache-closure.

Manual runs:

```bash
cargo run --package premath-cli -- traceability-check
cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json
cargo run --package premath-cli -- drift-budget-check --json
```

Runtime check execution, hook orchestration, provider-specific artifacts, and
instruction-envelope execution are outside Premath. Premath keeps deterministic
checker commands for kernel/coherence/traceability/local-check projection only.

## Premath CLI Surface

`premath-cli` includes checker and profile-bound projection operations. It
intentionally does not expose tracker mutation, workflow runner, or runtime witness
commands.
