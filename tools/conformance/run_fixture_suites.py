#!/usr/bin/env python3
"""Run executable conformance fixture suites with KCIR-style deterministic caching."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CACHE_DIR = ROOT / ".premath" / "cache" / "conformance"
CACHE_SCHEMA = 1
CACHE_SCHEME_ID = "kcir.cache.fixture-suite.v1"
CACHE_REF_PREFIX = "kcir1_"

DISABLED_ENV_VALUES = {"0", "false", "no", "off"}
ENABLED_ENV_VALUES = {"1", "true", "yes", "on"}

IGNORED_DIR_NAMES = {
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".git",
    ".jj",
    "target",
}


@dataclass(frozen=True)
class Suite:
    suite_id: str
    domain: str
    command: Tuple[str, ...]
    input_paths: Tuple[Path, ...]


@dataclass(frozen=True)
class SuitePlan:
    suite: Suite
    params_hash: str
    material_digest: str
    cache_binding: Dict[str, str]
    cache_ref: str
    cache_path: Path
    files_hashed: int


def resolve_rooted_path(path_text: str) -> Path:
    candidate = Path(path_text)
    if candidate.is_absolute():
        return candidate
    return ROOT / candidate


def unique_paths(paths: Sequence[Path]) -> Tuple[Path, ...]:
    out: List[Path] = []
    seen: set[str] = set()
    for path in paths:
        key = path.as_posix()
        if key in seen:
            continue
        seen.add(key)
        out.append(path)
    return tuple(out)


def load_coherence_contract_input_paths() -> Tuple[Path, ...]:
    contract_path = ROOT / "specs" / "premath" / "draft" / "COHERENCE-CONTRACT.json"
    base_paths: List[Path] = [
        ROOT / "tools" / "conformance" / "run_fixture_suites.py",
        ROOT / "tools" / "ci" / "control_plane_contract.py",
        contract_path,
        ROOT / "Cargo.toml",
        ROOT / "Cargo.lock",
        ROOT / "tests" / "conformance" / "fixtures" / "coherence-transport",
        ROOT / "tests" / "conformance" / "fixtures" / "coherence-site",
        ROOT / "crates" / "premath-kernel" / "src",
        ROOT / "crates" / "premath-coherence" / "src",
        ROOT / "crates" / "premath-cli" / "src" / "commands" / "coherence_check.rs",
    ]
    try:
        contract_payload = json.loads(contract_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return unique_paths(base_paths)

    surfaces = contract_payload.get("surfaces", {})
    if isinstance(surfaces, dict):
        for key, value in surfaces.items():
            if not isinstance(value, str):
                continue
            if key.endswith("Path") or key.endswith("Root"):
                base_paths.append(resolve_rooted_path(value))

    expected_operation_paths = contract_payload.get("expectedOperationPaths", [])
    if isinstance(expected_operation_paths, list):
        for value in expected_operation_paths:
            if isinstance(value, str):
                base_paths.append(resolve_rooted_path(value))

    overlay_docs = contract_payload.get("overlayDocs", [])
    if isinstance(overlay_docs, list):
        for value in overlay_docs:
            if isinstance(value, str) and value.strip():
                base_paths.append(
                    resolve_rooted_path(f"specs/premath/{value}.md")
                )

    return unique_paths(base_paths)


SUITES: Tuple[Suite, ...] = (
    Suite(
        suite_id="interop-core",
        domain="conformance.interop-core",
        command=("python3", "tools/conformance/run_interop_core_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_interop_core_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "interop-core",
        ),
    ),
    Suite(
        suite_id="gate",
        domain="conformance.gate",
        command=("python3", "tools/conformance/run_gate_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_gate_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "gate",
            ROOT / "tools" / "toy",
        ),
    ),
    Suite(
        suite_id="witness-id",
        domain="conformance.witness-id",
        command=("python3", "tools/conformance/run_witness_id_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_witness_id_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "witness-id",
            ROOT / "tools" / "toy" / "witness_id.py",
        ),
    ),
    Suite(
        suite_id="kernel-profile",
        domain="conformance.kernel-profile",
        command=("python3", "tools/conformance/run_kernel_profile_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_kernel_profile_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "kernel-profile",
            ROOT / "tests" / "toy" / "fixtures",
            ROOT / "tests" / "kcir_toy" / "fixtures",
            ROOT / "tools" / "toy",
            ROOT / "tools" / "kcir_toy",
        ),
    ),
    Suite(
        suite_id="doctrine-inf",
        domain="conformance.doctrine-inf",
        command=("python3", "tools/conformance/run_doctrine_inf_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_doctrine_inf_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "doctrine-inf",
            ROOT / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json",
        ),
    ),
    Suite(
        suite_id="coherence-contract",
        domain="conformance.coherence-contract",
        command=(
            "cargo",
            "run",
            "--package",
            "premath-cli",
            "--",
            "coherence-check",
            "--contract",
            "specs/premath/draft/COHERENCE-CONTRACT.json",
            "--repo-root",
            ".",
            "--json",
        ),
        input_paths=load_coherence_contract_input_paths(),
    ),
    Suite(
        suite_id="tusk-core",
        domain="conformance.tusk-core",
        command=("python3", "tools/conformance/run_tusk_core_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_tusk_core_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "tusk-core",
            ROOT / "crates" / "premath-tusk" / "src",
            ROOT / "crates" / "premath-cli" / "src" / "commands" / "tusk_eval.rs",
        ),
    ),
    Suite(
        suite_id="harness-typestate",
        domain="conformance.harness-typestate",
        command=("python3", "tools/conformance/run_harness_typestate_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_harness_typestate_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "harness-typestate",
            ROOT / "crates" / "premath-tusk" / "src" / "typestate.rs",
            ROOT / "crates" / "premath-cli" / "src" / "commands" / "harness_join_check.rs",
            ROOT / "crates" / "premath-cli" / "src" / "commands" / "mcp_serve.rs",
        ),
    ),
    Suite(
        suite_id="runtime-orchestration",
        domain="conformance.runtime-orchestration",
        command=("python3", "tools/conformance/run_runtime_orchestration_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_runtime_orchestration_vectors.py",
            ROOT / "tools" / "conformance" / "check_runtime_orchestration.py",
            ROOT / "tests" / "conformance" / "fixtures" / "runtime-orchestration",
            ROOT / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json",
            ROOT / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json",
            ROOT / "specs" / "premath" / "draft" / "HARNESS-RUNTIME.md",
            ROOT / "crates" / "premath-cli" / "src" / "commands" / "control_plane_gate.rs",
            ROOT / "tools" / "ci" / "governance_gate.py",
            ROOT / "tools" / "ci" / "kcir_mapping_gate.py",
        ),
    ),
    Suite(
        suite_id="capabilities",
        domain="conformance.capabilities",
        command=("python3", "tools/conformance/run_capability_vectors.py"),
        input_paths=(
            ROOT / "tools" / "conformance" / "run_fixture_suites.py",
            ROOT / "tools" / "conformance" / "run_capability_vectors.py",
            ROOT / "tests" / "conformance" / "fixtures" / "capabilities",
            ROOT / "tools" / "ci",
            ROOT / "policies" / "instruction",
        ),
    ),
)


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def to_rel_path(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def iter_files(path: Path) -> Iterable[Path]:
    if path.is_file():
        yield path
        return
    if not path.is_dir():
        return
    for candidate in sorted(path.rglob("*")):
        if not candidate.is_file():
            continue
        if any(part in IGNORED_DIR_NAMES for part in candidate.parts):
            continue
        yield candidate


def compute_material_digest(paths: Sequence[Path]) -> Tuple[str, int]:
    entries: List[Dict[str, Any]] = []
    file_count = 0
    for path in paths:
        rel_path = to_rel_path(path)
        if not path.exists():
            entries.append({"path": rel_path, "kind": "missing"})
            continue
        if path.is_file():
            file_sha = hashlib.sha256(path.read_bytes()).hexdigest()
            entries.append({"path": rel_path, "kind": "file", "sha256": file_sha})
            file_count += 1
            continue
        if not path.is_dir():
            entries.append({"path": rel_path, "kind": "unknown"})
            continue

        entries.append({"path": rel_path, "kind": "dir"})
        hashed_any = False
        for file_path in iter_files(path):
            file_sha = hashlib.sha256(file_path.read_bytes()).hexdigest()
            entries.append({"path": to_rel_path(file_path), "kind": "file", "sha256": file_sha})
            file_count += 1
            hashed_any = True
        if not hashed_any:
            entries.append({"path": rel_path, "kind": "empty_dir"})

    return stable_hash(entries), file_count


def compute_params_hash(suite: Suite) -> str:
    return stable_hash(
        {
            "suiteId": suite.suite_id,
            "domain": suite.domain,
            "command": list(suite.command),
            "python": ".".join(str(part) for part in sys.version_info[:3]),
            "cacheSchema": CACHE_SCHEMA,
            "cacheSchemeId": CACHE_SCHEME_ID,
        }
    )


def make_suite_plan(suite: Suite, cache_dir: Path) -> SuitePlan:
    params_hash = compute_params_hash(suite)
    material_digest, files_hashed = compute_material_digest(suite.input_paths)
    binding = {
        "schemeId": CACHE_SCHEME_ID,
        "domain": suite.domain,
        "paramsHash": params_hash,
        "digest": material_digest,
    }
    cache_ref = CACHE_REF_PREFIX + stable_hash(binding)
    cache_path = cache_dir / suite.suite_id / f"{cache_ref}.json"
    return SuitePlan(
        suite=suite,
        params_hash=params_hash,
        material_digest=material_digest,
        cache_binding=binding,
        cache_ref=cache_ref,
        cache_path=cache_path,
        files_hashed=files_hashed,
    )


def cache_enabled(no_cache_flag: bool) -> bool:
    if no_cache_flag:
        return False
    raw = os.environ.get("PREMATH_CONFORMANCE_CACHE")
    if raw is None or not raw.strip():
        return True
    normalized = raw.strip().lower()
    if normalized in DISABLED_ENV_VALUES:
        return False
    if normalized in ENABLED_ENV_VALUES:
        return True
    return True


def load_cache_hit(path: Path, expected_ref: str) -> bool:
    if not path.exists():
        return False
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    if not isinstance(payload, dict):
        return False
    if payload.get("schema") != CACHE_SCHEMA:
        return False
    if payload.get("cacheRef") != expected_ref:
        return False
    return payload.get("result") == "passed"


def write_cache_hit(plan: SuitePlan, duration_ms: int) -> None:
    payload = {
        "schema": CACHE_SCHEMA,
        "cacheKind": "conformance.fixture-suite.kcir.v1",
        "suiteId": plan.suite.suite_id,
        "cacheRef": plan.cache_ref,
        "cacheBinding": plan.cache_binding,
        "command": list(plan.suite.command),
        "filesHashed": plan.files_hashed,
        "durationMs": duration_ms,
        "result": "passed",
        "createdAt": datetime.now(timezone.utc).isoformat(),
    }
    plan.cache_path.parent.mkdir(parents=True, exist_ok=True)
    with plan.cache_path.open("w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, ensure_ascii=False)
        f.write("\n")


def run_suite(plan: SuitePlan) -> Tuple[int, int]:
    print(
        f"[conformance-run] RUN suite={plan.suite.suite_id} "
        f"files={plan.files_hashed} ref={plan.cache_ref}"
    )
    started = datetime.now(timezone.utc)
    completed = subprocess.run(list(plan.suite.command), cwd=ROOT)
    duration_ms = int((datetime.now(timezone.utc) - started).total_seconds() * 1000)
    if completed.returncode == 0:
        print(f"[conformance-run] OK suite={plan.suite.suite_id} durationMs={duration_ms}")
    else:
        print(
            f"[conformance-run] FAIL suite={plan.suite.suite_id} "
            f"durationMs={duration_ms} exit={completed.returncode}"
        )
    return int(completed.returncode), duration_ms


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run conformance fixture suites with deterministic cache bindings."
    )
    parser.add_argument(
        "--suite",
        action="append",
        default=None,
        choices=[suite.suite_id for suite in SUITES],
        help="Suite ID to execute (repeatable). Default: all.",
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=DEFAULT_CACHE_DIR,
        help=f"Cache directory (default: {DEFAULT_CACHE_DIR})",
    )
    parser.add_argument(
        "--no-cache",
        action="store_true",
        help="Disable cache reads/writes for this invocation.",
    )
    parser.add_argument(
        "--print-plan",
        action="store_true",
        help="Print computed cache plan material before execution.",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    selected = set(args.suite or [])
    suites = [suite for suite in SUITES if not selected or suite.suite_id in selected]
    cache_dir = args.cache_dir
    if not cache_dir.is_absolute():
        cache_dir = (ROOT / cache_dir).resolve()
    use_cache = cache_enabled(args.no_cache)

    plans = [make_suite_plan(suite, cache_dir) for suite in suites]

    if args.print_plan:
        payload = {
            "schema": 1,
            "cacheEnabled": use_cache,
            "cacheDir": to_rel_path(cache_dir),
            "suites": [
                {
                    "suiteId": plan.suite.suite_id,
                    "domain": plan.suite.domain,
                    "cacheRef": plan.cache_ref,
                    "cacheBinding": plan.cache_binding,
                    "filesHashed": plan.files_hashed,
                    "cachePath": to_rel_path(plan.cache_path),
                }
                for plan in plans
            ],
        }
        print(canonical_json(payload))

    cache_hits = 0
    executed = 0
    for plan in plans:
        if use_cache and load_cache_hit(plan.cache_path, plan.cache_ref):
            cache_hits += 1
            print(f"[conformance-run] HIT suite={plan.suite.suite_id} ref={plan.cache_ref}")
            continue
        exit_code, duration_ms = run_suite(plan)
        executed += 1
        if exit_code != 0:
            return exit_code
        if use_cache:
            write_cache_hit(plan, duration_ms=duration_ms)
            print(f"[conformance-run] STORE suite={plan.suite.suite_id} ref={plan.cache_ref}")

    print(
        f"[conformance-run] SUMMARY suites={len(plans)} "
        f"executed={executed} cacheHits={cache_hits} cacheEnabled={str(use_cache).lower()}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
