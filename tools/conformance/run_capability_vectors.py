#!/usr/bin/env python3
"""
Execute capability conformance vectors.

Executable capability defaults are loaded from:
- specs/premath/draft/CAPABILITY-REGISTRY.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

from change_projection import project_required_checks  # type: ignore  # noqa: E402
from instruction_check_client import InstructionCheckError, run_instruction_check  # type: ignore  # noqa: E402
from proposal_check_client import (  # type: ignore  # noqa: E402
    ProposalCheckError,
    run_proposal_check,
)
from provider_env import map_github_to_premath_env, resolve_premath_ci_refs  # type: ignore  # noqa: E402
from required_witness import verify_required_witness_payload  # type: ignore  # noqa: E402

CAPABILITY_NORMAL_FORMS = "capabilities.normal_forms"
CAPABILITY_KCIR_WITNESSES = "capabilities.kcir_witnesses"
CAPABILITY_COMMITMENT_CHECKPOINTS = "capabilities.commitment_checkpoints"
CAPABILITY_SQUEAK_SITE = "capabilities.squeak_site"
CAPABILITY_CI_WITNESSES = "capabilities.ci_witnesses"
CAPABILITY_INSTRUCTION_TYPING = "capabilities.instruction_typing"
CAPABILITY_ADJOINTS_SITES = "capabilities.adjoints_sites"
CAPABILITY_CHANGE_MORPHISMS = "capabilities.change_morphisms"
CAPABILITY_REGISTRY_KIND = "premath.capability_registry.v1"
CAPABILITY_REGISTRY_PATH = ROOT / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json"
BLOCKING_DEP_TYPES = {
    "blocks",
    "parent-child",
    "conditional-blocks",
    "waits-for",
}
DEFAULT_LEASE_TTL_SECONDS = 3600
MIN_LEASE_TTL_SECONDS = 30
MAX_LEASE_TTL_SECONDS = 86_400
ADJOINTS_SITES_REQUIRED_OBLIGATIONS = {
    "adjoint_triangle",
    "beck_chevalley_sigma",
    "beck_chevalley_pi",
    "refinement_invariance",
}
REQUIRED_CROSS_LANE_ROUTE = "span_square_commutation"
OBSTRUCTION_CLASS_TO_CONSTRUCTOR: Dict[str, Tuple[str, str, str]] = {
    "stability_failure": ("semantic", "stability", "stability_failure"),
    "locality_failure": ("semantic", "locality", "locality_failure"),
    "descent_failure": ("semantic", "descent", "descent_failure"),
    "glue_non_contractible": ("semantic", "contractibility", "glue_non_contractible"),
    "adjoint_triple_coherence_failure": (
        "semantic",
        "adjoint_triple",
        "adjoint_triple_coherence_failure",
    ),
    "coherence.cwf_substitution_identity.violation": (
        "structural",
        "cwf_substitution_identity",
        "coherence.cwf_substitution_identity.violation",
    ),
    "coherence.cwf_substitution_composition.violation": (
        "structural",
        "cwf_substitution_composition",
        "coherence.cwf_substitution_composition.violation",
    ),
    "coherence.span_square_commutation.violation": (
        "commutation",
        "span_square_commutation",
        "coherence.span_square_commutation.violation",
    ),
    "decision_witness_sha_mismatch": (
        "lifecycle",
        "decision_attestation",
        "decision_witness_sha_mismatch",
    ),
    "decision_delta_sha_mismatch": (
        "lifecycle",
        "decision_delta_attestation",
        "decision_delta_sha_mismatch",
    ),
    "unification.evidence_factorization.missing": (
        "lifecycle",
        "evidence_factorization_missing",
        "unification.evidence_factorization.missing",
    ),
    "unification.evidence_factorization.ambiguous": (
        "lifecycle",
        "evidence_factorization_ambiguous",
        "unification.evidence_factorization.ambiguous",
    ),
    "unification.evidence_factorization.unbound": (
        "lifecycle",
        "evidence_factorization_unbound",
        "unification.evidence_factorization.unbound",
    ),
}
OBSTRUCTION_CONSTRUCTOR_TO_CANONICAL: Dict[Tuple[str, str], str] = {
    (family, tag): canonical
    for family, tag, canonical in OBSTRUCTION_CLASS_TO_CONSTRUCTOR.values()
}


@dataclass(frozen=True)
class VectorOutcome:
    result: str
    kernel_verdict: str
    gate_failure_classes: List[str]
    cmp_ref: Optional[str] = None


def load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError(f"json root must be object: {path}")
    return data


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def compute_typed_core_projection_digest(
    authority_payload_digest: str,
    normalizer_id: str,
    policy_digest: str,
) -> str:
    h = hashlib.sha256()
    for part in (authority_payload_digest, normalizer_id, policy_digest):
        h.update(part.encode("utf-8"))
        h.update(b"\x00")
    return "ev1_" + h.hexdigest()


def normalize_semantics(value: Any) -> Any:
    if isinstance(value, dict):
        return {k: normalize_semantics(value[k]) for k in sorted(value)}
    if isinstance(value, list):
        normalized_items = [normalize_semantics(v) for v in value]
        dedup: Dict[str, Any] = {}
        for item in normalized_items:
            dedup[canonical_json(item)] = item
        return [dedup[k] for k in sorted(dedup.keys())]
    return value


def compute_cmp_ref(semantic: Any, normalizer_id: str, policy_digest: str) -> str:
    material = {
        "semantic": normalize_semantics(semantic),
        "normalizerId": normalizer_id,
        "policyDigest": policy_digest,
    }
    return "cmp1_" + stable_hash(material)


def compute_kcir_ref(payload: Any) -> str:
    return "kcir1_" + stable_hash(payload)


def compute_run_material_ref(run_material: Any) -> str:
    return "run1_" + stable_hash(run_material)


def compute_checkpoint_ref(checkpoint_payload: Any) -> str:
    return "ckpt1_" + stable_hash(checkpoint_payload)


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label} must be a non-empty string")
    return value


def ensure_int(value: Any, label: str) -> int:
    if not isinstance(value, int):
        raise ValueError(f"{label} must be an integer")
    return value


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str):
            raise ValueError(f"{label}[{idx}] must be a string")
        out.append(item)
    return out


def ensure_string_mapping(value: Any, label: str) -> Dict[str, str]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object")
    out: Dict[str, str] = {}
    for key, item in value.items():
        if not isinstance(key, str) or not key:
            raise ValueError(f"{label} keys must be non-empty strings")
        if not isinstance(item, str):
            raise ValueError(f"{label}[{key!r}] must be a string")
        out[key] = item
    return out


def ensure_gate_witness_payloads(
    value: Any,
    label: str,
) -> Optional[Dict[str, Dict[str, Any]]]:
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object")
    out: Dict[str, Dict[str, Any]] = {}
    for key, payload in value.items():
        if not isinstance(key, str) or not key:
            raise ValueError(f"{label} keys must be non-empty strings")
        if not isinstance(payload, dict):
            raise ValueError(f"{label}[{key!r}] must be an object")
        out[key] = payload
    return out


def load_executable_capabilities(registry_path: Path) -> List[str]:
    payload = load_json(registry_path)
    if payload.get("schema") != 1:
        raise ValueError(f"{registry_path}: schema must be 1")
    kind = payload.get("registryKind")
    if not isinstance(kind, str) or kind != CAPABILITY_REGISTRY_KIND:
        raise ValueError(
            f"{registry_path}: registryKind must be {CAPABILITY_REGISTRY_KIND!r}, got {kind!r}"
        )
    raw = payload.get("executableCapabilities")
    if not isinstance(raw, list) or not raw:
        raise ValueError(f"{registry_path}: executableCapabilities must be a non-empty list")
    out: List[str] = []
    seen: set[str] = set()
    for idx, item in enumerate(raw):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(
                f"{registry_path}: executableCapabilities[{idx}] must be a non-empty string"
            )
        capability_id = item.strip()
        if capability_id in seen:
            raise ValueError(
                f"{registry_path}: executableCapabilities contains duplicate {capability_id!r}"
            )
        seen.add(capability_id)
        out.append(capability_id)
    return out


def lease_token(value: str) -> str:
    out_chars: List[str] = []
    for ch in value:
        if ch.isascii() and ch.isalnum():
            out_chars.append(ch.lower())
        elif ch in {"-", "_"}:
            out_chars.append(ch)
        else:
            out_chars.append("_")
    trimmed = "".join(out_chars).strip("_")
    return trimmed if trimmed else "anon"


def resolve_lease_id(raw_lease_id: Any, issue_id: str, assignee: str) -> str:
    if isinstance(raw_lease_id, str) and raw_lease_id:
        return raw_lease_id
    return f"lease1_{lease_token(issue_id)}_{lease_token(assignee)}"


def resolve_lease_expiry_unix_ms(
    now_unix_ms: int,
    ttl_seconds_raw: Any,
    expires_at_unix_ms_raw: Any,
) -> tuple[Optional[int], Optional[str]]:
    if ttl_seconds_raw is not None and not isinstance(ttl_seconds_raw, int):
        raise ValueError("leaseTtlSeconds must be an integer when present")
    if expires_at_unix_ms_raw is not None and not isinstance(expires_at_unix_ms_raw, int):
        raise ValueError("leaseExpiresAtUnixMs must be an integer when present")

    if ttl_seconds_raw is not None and expires_at_unix_ms_raw is not None:
        return (None, "lease_binding_ambiguous")

    if expires_at_unix_ms_raw is not None:
        if expires_at_unix_ms_raw <= now_unix_ms:
            return (None, "lease_invalid_expires_at")
        return (expires_at_unix_ms_raw, None)

    ttl_seconds = ttl_seconds_raw if isinstance(ttl_seconds_raw, int) else DEFAULT_LEASE_TTL_SECONDS
    if ttl_seconds < MIN_LEASE_TTL_SECONDS or ttl_seconds > MAX_LEASE_TTL_SECONDS:
        return (None, "lease_invalid_ttl")
    return (now_unix_ms + ttl_seconds * 1000, None)


def evaluate_nf_binding_stable(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_NORMAL_FORMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    semantic = artifacts.get("input")
    runs = artifacts.get("runs")
    if not isinstance(runs, list) or len(runs) < 2:
        raise ValueError("artifacts.runs must be a list of at least 2 runs")

    normalizers: List[str] = []
    policies: List[str] = []
    cmp_refs: List[str] = []
    for idx, run in enumerate(runs):
        if not isinstance(run, dict):
            raise ValueError(f"artifacts.runs[{idx}] must be an object")
        normalizer_id = ensure_string(run.get("normalizerId"), f"artifacts.runs[{idx}].normalizerId")
        policy_digest = ensure_string(run.get("policyDigest"), f"artifacts.runs[{idx}].policyDigest")
        normalizers.append(normalizer_id)
        policies.append(policy_digest)
        cmp_refs.append(compute_cmp_ref(semantic, normalizer_id, policy_digest))

    same_normalizer = len(set(normalizers)) == 1
    same_policy = len(set(policies)) == 1
    same_cmp = len(set(cmp_refs)) == 1

    if same_normalizer and same_policy and same_cmp:
        return VectorOutcome("accepted", "accepted", [], cmp_ref=cmp_refs[0])
    return VectorOutcome("rejected", "rejected", ["nf_binding_unstable"])


def evaluate_nf_equiv_accept(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    left = artifacts.get("left")
    right = artifacts.get("right")
    if not isinstance(left, dict) or not isinstance(right, dict):
        raise ValueError("artifacts.left and artifacts.right must be objects")

    left_normalizer = ensure_string(left.get("normalizerId"), "artifacts.left.normalizerId")
    right_normalizer = ensure_string(right.get("normalizerId"), "artifacts.right.normalizerId")
    left_policy = ensure_string(left.get("policyDigest"), "artifacts.left.policyDigest")
    right_policy = ensure_string(right.get("policyDigest"), "artifacts.right.policyDigest")

    if left_normalizer != right_normalizer or left_policy != right_policy:
        return VectorOutcome("rejected", "rejected", ["nf_policy_binding_mismatch"])

    left_cmp = compute_cmp_ref(left.get("semantic"), left_normalizer, left_policy)
    right_cmp = compute_cmp_ref(right.get("semantic"), right_normalizer, right_policy)
    if left_cmp == right_cmp:
        return VectorOutcome("accepted", "accepted", [], cmp_ref=left_cmp)
    return VectorOutcome("rejected", "rejected", ["nf_not_equivalent"])


def evaluate_nf_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "normalized" and CAPABILITY_NORMAL_FORMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_nf_policy_binding_mismatch(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    left = artifacts.get("left")
    right = artifacts.get("right")
    if not isinstance(left, dict) or not isinstance(right, dict):
        raise ValueError("artifacts.left and artifacts.right must be objects")

    left_normalizer = ensure_string(left.get("normalizerId"), "artifacts.left.normalizerId")
    right_normalizer = ensure_string(right.get("normalizerId"), "artifacts.right.normalizerId")
    left_policy = ensure_string(left.get("policyDigest"), "artifacts.left.policyDigest")
    right_policy = ensure_string(right.get("policyDigest"), "artifacts.right.policyDigest")

    if left_normalizer != right_normalizer or left_policy != right_policy:
        return VectorOutcome("rejected", "rejected", ["nf_policy_binding_mismatch"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_nf_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    cmp_ref: Optional[str] = None
    if profile == "normalized":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_NORMAL_FORMS not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
        normalizer_id = ensure_string(artifacts.get("normalizerId"), "artifacts.normalizerId")
        policy_digest = ensure_string(artifacts.get("policyDigest"), "artifacts.policyDigest")
        cmp_ref = compute_cmp_ref(input_data.get("semantic"), normalizer_id, policy_digest)

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes, cmp_ref=cmp_ref)


def evaluate_normal_forms_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/nf_binding_stable":
        return evaluate_nf_binding_stable(case)
    if vector_id == "golden/nf_equiv_accept":
        return evaluate_nf_equiv_accept(case)
    if vector_id == "adversarial/nf_requires_claim":
        return evaluate_nf_requires_claim(case)
    if vector_id == "adversarial/nf_policy_binding_mismatch":
        return evaluate_nf_policy_binding_mismatch(case)
    if vector_id.startswith("invariance/"):
        return evaluate_nf_invariance(case)
    raise ValueError(f"unsupported normal_forms vector id: {vector_id}")


def validate_manifest_vectors(capability_dir: Path, errors: List[str]) -> Optional[List[str]]:
    manifest = load_json(capability_dir / "manifest.json")
    vectors_raw = manifest.get("vectors")
    if not isinstance(vectors_raw, list) or not vectors_raw:
        errors.append(f"{capability_dir}/manifest.json: vectors must be a non-empty list")
        return None

    vectors = [v for v in vectors_raw if isinstance(v, str) and v]
    if len(vectors) != len(vectors_raw):
        errors.append(f"{capability_dir}/manifest.json: all vectors must be non-empty strings")
        return None
    return vectors


def run_capability_vectors(
    capability_dir: Path,
    evaluator,
    errors: List[str],
) -> Tuple[int, int]:
    vectors = validate_manifest_vectors(capability_dir, errors)
    if vectors is None:
        return (0, 0)

    invariance_groups: Dict[str, List[Tuple[str, VectorOutcome]]] = {}
    checked = 0

    for vector in vectors:
        case_path = capability_dir / vector / "case.json"
        expect_path = capability_dir / vector / "expect.json"
        try:
            case = load_json(case_path)
            expect = load_json(expect_path)
            outcome = evaluator(vector, case)
            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if outcome.result != expected_result:
                errors.append(
                    f"{capability_dir.name}/{vector}: result mismatch "
                    f"(expected={expected_result}, actual={outcome.result})"
                )
                continue
            expected_failure_classes_raw = expect.get("expectedFailureClasses")
            if expected_failure_classes_raw is not None:
                expected_failure_classes = sorted(
                    set(
                        ensure_string_list(
                            expected_failure_classes_raw,
                            f"{expect_path}: expectedFailureClasses",
                        )
                    )
                )
                actual_failure_classes = sorted(set(outcome.gate_failure_classes))
                if actual_failure_classes != expected_failure_classes:
                    errors.append(
                        f"{capability_dir.name}/{vector}: failure class mismatch "
                        f"(expected={expected_failure_classes}, actual={actual_failure_classes})"
                    )
                    continue

            if vector.startswith("invariance/"):
                scenario_id = ensure_string(
                    case.get("semanticScenarioId"), f"{case_path}: semanticScenarioId"
                )
                invariance_groups.setdefault(scenario_id, []).append((vector, outcome))
        except Exception as exc:
            errors.append(f"{capability_dir.name}/{vector}: {exc}")
            continue

        checked += 1
        print(f"[ok] {capability_dir.name}/{vector}")

    for scenario_id, rows in sorted(invariance_groups.items()):
        if len(rows) != 2:
            errors.append(
                f"{capability_dir.name}: invariance scenario '{scenario_id}' must have 2 vectors, found {len(rows)}"
            )
            continue
        (_, left), (_, right) = rows
        if left.kernel_verdict != right.kernel_verdict:
            errors.append(
                f"{capability_dir.name}: invariance '{scenario_id}' kernelVerdict mismatch "
                f"({left.kernel_verdict} vs {right.kernel_verdict})"
            )
        if sorted(left.gate_failure_classes) != sorted(right.gate_failure_classes):
            errors.append(
                f"{capability_dir.name}: invariance '{scenario_id}' Gate failure class mismatch "
                f"({left.gate_failure_classes} vs {right.gate_failure_classes})"
            )

    return (1, checked)


def run_normal_forms(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_normal_forms_vector, errors)


def verify_kcir_bundle_refs(
    witness_bundle: Dict[str, Any],
    ref_store: Dict[str, Any],
) -> Optional[str]:
    refs = ensure_string_list(witness_bundle.get("refs"), "artifacts.witnessBundle.refs")
    if not refs:
        raise ValueError("artifacts.witnessBundle.refs must be non-empty")

    for ref in refs:
        if ref not in ref_store:
            return "kcir_ref_missing"
        payload = ref_store[ref]
        if not isinstance(payload, dict):
            raise ValueError(f"artifacts.refStore[{ref}] must be an object payload")
        if compute_kcir_ref(payload) != ref:
            return "kcir_ref_tampered"

    return None


def evaluate_kcir_refs_resolve(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_KCIR_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    witness_bundle = artifacts.get("witnessBundle")
    ref_store = artifacts.get("refStore")
    if not isinstance(witness_bundle, dict):
        raise ValueError("artifacts.witnessBundle must be an object")
    if not isinstance(ref_store, dict):
        raise ValueError("artifacts.refStore must be an object")

    failure = verify_kcir_bundle_refs(witness_bundle, ref_store)
    if failure is None:
        return VectorOutcome("accepted", "accepted", [])
    return VectorOutcome("rejected", "rejected", [failure])


def evaluate_kcir_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "kcir_linked_witness" and CAPABILITY_KCIR_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_kcir_tampered_ref_reject(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_KCIR_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    witness_bundle = artifacts.get("witnessBundle")
    ref_store = artifacts.get("refStore")
    if not isinstance(witness_bundle, dict):
        raise ValueError("artifacts.witnessBundle must be an object")
    if not isinstance(ref_store, dict):
        raise ValueError("artifacts.refStore must be an object")

    failure = verify_kcir_bundle_refs(witness_bundle, ref_store)
    if failure is None:
        return VectorOutcome("accepted", "accepted", [])
    return VectorOutcome("rejected", "rejected", [failure])


def evaluate_kcir_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile == "kcir_linked_witness":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_KCIR_WITNESSES not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

        witness_bundle = artifacts.get("witnessBundle")
        ref_store = artifacts.get("refStore")
        if not isinstance(witness_bundle, dict):
            raise ValueError("artifacts.witnessBundle must be an object")
        if not isinstance(ref_store, dict):
            raise ValueError("artifacts.refStore must be an object")

        failure = verify_kcir_bundle_refs(witness_bundle, ref_store)
        if failure is not None:
            return VectorOutcome("rejected", "rejected", [failure])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_kcir_witness_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/kcir_witness_refs_resolve":
        return evaluate_kcir_refs_resolve(case)
    if vector_id == "adversarial/kcir_witness_requires_claim":
        return evaluate_kcir_requires_claim(case)
    if vector_id == "adversarial/kcir_witness_tampered_ref_reject":
        return evaluate_kcir_tampered_ref_reject(case)
    if vector_id.startswith("invariance/"):
        return evaluate_kcir_invariance(case)
    raise ValueError(f"unsupported kcir_witnesses vector id: {vector_id}")


def run_kcir_witnesses(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_kcir_witness_vector, errors)


def verify_checkpoint_binding(run_material: Any, checkpoint: Any) -> Optional[str]:
    if not isinstance(run_material, dict):
        raise ValueError("artifacts.runMaterial must be an object")
    if not isinstance(checkpoint, dict):
        raise ValueError("artifacts.checkpoint must be an object")

    declared_run_ref = ensure_string(checkpoint.get("runMaterialRef"), "artifacts.checkpoint.runMaterialRef")
    expected_run_ref = compute_run_material_ref(run_material)
    if declared_run_ref != expected_run_ref:
        return "checkpoint_run_material_ref_mismatch"

    declared_checkpoint_ref = ensure_string(checkpoint.get("checkpointRef"), "artifacts.checkpoint.checkpointRef")
    checkpoint_body = dict(checkpoint)
    checkpoint_body.pop("checkpointRef", None)
    expected_checkpoint_ref = compute_checkpoint_ref(checkpoint_body)
    if declared_checkpoint_ref != expected_checkpoint_ref:
        return "checkpoint_ref_mismatch"

    return None


def evaluate_checkpoint_create_verify_ok(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_COMMITMENT_CHECKPOINTS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    failure = verify_checkpoint_binding(artifacts.get("runMaterial"), artifacts.get("checkpoint"))
    if failure is None:
        return VectorOutcome("accepted", "accepted", [])
    return VectorOutcome("rejected", "rejected", [failure])


def evaluate_checkpoint_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "checkpoint_enabled" and CAPABILITY_COMMITMENT_CHECKPOINTS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_checkpoint_tampered_or_mismatch(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_COMMITMENT_CHECKPOINTS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    failure = verify_checkpoint_binding(artifacts.get("runMaterial"), artifacts.get("checkpoint"))
    if failure is None:
        return VectorOutcome("accepted", "accepted", [])
    return VectorOutcome("rejected", "rejected", [failure])


def evaluate_checkpoint_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile == "checkpoint_enabled":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_COMMITMENT_CHECKPOINTS not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

        failure = verify_checkpoint_binding(artifacts.get("runMaterial"), artifacts.get("checkpoint"))
        if failure is not None:
            return VectorOutcome("rejected", "rejected", [failure])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_commitment_checkpoint_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/checkpoint_create_verify_ok":
        return evaluate_checkpoint_create_verify_ok(case)
    if vector_id == "adversarial/checkpoint_requires_claim":
        return evaluate_checkpoint_requires_claim(case)
    if vector_id == "adversarial/checkpoint_tampered_or_mismatch":
        return evaluate_checkpoint_tampered_or_mismatch(case)
    if vector_id.startswith("invariance/"):
        return evaluate_checkpoint_invariance(case)
    raise ValueError(f"unsupported commitment_checkpoints vector id: {vector_id}")


def run_commitment_checkpoints(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_commitment_checkpoint_vector, errors)


def canonical_loc_descriptor(descriptor: Dict[str, Any], label: str) -> Dict[str, Any]:
    world_id = ensure_string(descriptor.get("worldId"), f"{label}.worldId")
    runtime_profile = ensure_string(descriptor.get("runtimeProfile"), f"{label}.runtimeProfile")
    substrate_binding_ref = ensure_string(descriptor.get("substrateBindingRef"), f"{label}.substrateBindingRef")
    capability_vector = sorted(
        set(ensure_string_list(descriptor.get("capabilityVector", []), f"{label}.capabilityVector"))
    )
    return {
        "worldId": world_id,
        "runtimeProfile": runtime_profile,
        "capabilityVector": capability_vector,
        "substrateBindingRef": substrate_binding_ref,
    }


def compute_site_loc_ref(descriptor: Dict[str, Any], label: str) -> str:
    canonical_descriptor = canonical_loc_descriptor(descriptor, label)
    return "loc1_" + stable_hash(canonical_descriptor)


def canonical_overlap_section(section: Dict[str, Any], label: str) -> Dict[str, Any]:
    kernel_verdict = ensure_string(section.get("kernelVerdict"), f"{label}.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError(f"{label}.kernelVerdict must be 'accepted' or 'rejected'")

    gate_failure_classes = sorted(
        ensure_string_list(section.get("gateFailureClasses", []), f"{label}.gateFailureClasses")
    )
    required_checks = sorted(
        set(ensure_string_list(section.get("requiredChecks", []), f"{label}.requiredChecks"))
    )
    policy_digest = ensure_string(section.get("policyDigest"), f"{label}.policyDigest")
    projection_digest = ensure_string(section.get("projectionDigest"), f"{label}.projectionDigest")

    return {
        "kernelVerdict": kernel_verdict,
        "gateFailureClasses": gate_failure_classes,
        "requiredChecks": required_checks,
        "policyDigest": policy_digest,
        "projectionDigest": projection_digest,
    }


def evaluate_site_loc_descriptor_deterministic(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    left = artifacts.get("leftDescriptor")
    right = artifacts.get("rightDescriptor")
    if not isinstance(left, dict) or not isinstance(right, dict):
        raise ValueError("artifacts.leftDescriptor and artifacts.rightDescriptor must be objects")

    left_ref = compute_site_loc_ref(left, "artifacts.leftDescriptor")
    right_ref = compute_site_loc_ref(right, "artifacts.rightDescriptor")
    if left_ref == right_ref:
        return VectorOutcome("accepted", "accepted", [], cmp_ref=left_ref)
    return VectorOutcome("rejected", "rejected", ["site_loc_descriptor_mismatch"])


def evaluate_site_overlap_agreement(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    left = artifacts.get("leftSection")
    right = artifacts.get("rightSection")
    if not isinstance(left, dict) or not isinstance(right, dict):
        raise ValueError("artifacts.leftSection and artifacts.rightSection must be objects")

    left_section = canonical_overlap_section(left, "artifacts.leftSection")
    right_section = canonical_overlap_section(right, "artifacts.rightSection")
    if left_section == right_section:
        return VectorOutcome("accepted", "accepted", [])
    return VectorOutcome("rejected", "rejected", ["site_overlap_mismatch"])


def evaluate_site_glue_non_contractible(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    mode = artifacts.get("mode")
    if not isinstance(mode, dict):
        raise ValueError("artifacts.mode must be an object")
    ensure_string(mode.get("normalizerId"), "artifacts.mode.normalizerId")
    ensure_string(mode.get("policyDigest"), "artifacts.mode.policyDigest")

    glue_proposals = artifacts.get("glueProposals")
    if not isinstance(glue_proposals, list):
        raise ValueError("artifacts.glueProposals must be a list")
    if not glue_proposals:
        return VectorOutcome("rejected", "rejected", ["site_glue_missing"])

    fingerprints: List[str] = []
    for idx, proposal in enumerate(glue_proposals):
        if not isinstance(proposal, dict):
            raise ValueError(f"artifacts.glueProposals[{idx}] must be an object")
        fingerprints.append(stable_hash(normalize_semantics(proposal)))

    if len(set(fingerprints)) == 1:
        return VectorOutcome("accepted", "accepted", [])
    return VectorOutcome("rejected", "rejected", ["site_glue_non_contractible"])


def evaluate_site_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "site_linked_runtime_evidence" and CAPABILITY_SQUEAK_SITE not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_site_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_SQUEAK_SITE not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
        descriptor = artifacts.get("locationDescriptor")
        if not isinstance(descriptor, dict):
            raise ValueError("artifacts.locationDescriptor must be an object")
        compute_site_loc_ref(descriptor, "artifacts.locationDescriptor")

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_squeak_site_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/site_loc_descriptor_deterministic":
        return evaluate_site_loc_descriptor_deterministic(case)
    if vector_id == "golden/site_overlap_agreement_accept":
        return evaluate_site_overlap_agreement(case)
    if vector_id == "adversarial/site_overlap_mismatch_reject":
        return evaluate_site_overlap_agreement(case)
    if vector_id == "adversarial/site_glue_missing_reject":
        return evaluate_site_glue_non_contractible(case)
    if vector_id == "adversarial/site_glue_non_contractible_reject":
        return evaluate_site_glue_non_contractible(case)
    if vector_id == "adversarial/site_requires_claim":
        return evaluate_site_requires_claim(case)
    if vector_id.startswith("invariance/"):
        return evaluate_site_invariance(case)
    raise ValueError(f"unsupported squeak_site vector id: {vector_id}")


def run_squeak_site(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_squeak_site_vector, errors)


def canonical_instruction_envelope(instruction: Dict[str, Any]) -> Dict[str, Any]:
    payload: Dict[str, Any] = dict(instruction)
    payload.setdefault("schema", 1)
    payload.setdefault("typingPolicy", {"allowUnknown": False})
    payload.setdefault("capabilityClaims", [])
    with tempfile.TemporaryDirectory(prefix="premath-capability-instruction-check-") as tmp:
        instruction_path = Path(tmp) / "20260222T000000Z-capability-check.json"
        instruction_path.write_text(
            json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False),
            encoding="utf-8",
        )
        try:
            validated = run_instruction_check(ROOT, instruction_path)
        except InstructionCheckError as exc:
            raise ValueError(f"{exc.failure_class}: {exc.reason}") from exc
    return {
        "intent": validated["intent"],
        "scope": validated["scope"],
        "normalizerId": validated["normalizerId"],
        "policyDigest": validated["policyDigest"],
        "requestedChecks": validated["requestedChecks"],
    }


def compute_instruction_digest(instruction: Dict[str, Any]) -> str:
    return "instr1_" + stable_hash(canonical_instruction_envelope(instruction))


def checked_llm_proposal(proposal: Dict[str, Any]) -> Tuple[Optional[Dict[str, Any]], Optional[str]]:
    try:
        return run_proposal_check(ROOT, proposal), None
    except ProposalCheckError as exc:
        return None, exc.failure_class


def canonical_instruction_classification(classification: Dict[str, Any], label: str) -> Dict[str, str]:
    state = ensure_string(classification.get("state"), f"{label}.state")
    if state == "typed":
        kind = ensure_string(classification.get("kind"), f"{label}.kind")
        return {
            "state": state,
            "kind": kind,
        }
    if state == "unknown":
        reason = ensure_string(classification.get("reason"), f"{label}.reason")
        return {
            "state": state,
            "reason": reason,
        }
    raise ValueError(f"{label}.state must be 'typed' or 'unknown'")


def evaluate_instruction_typed_deterministic(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_INSTRUCTION_TYPING not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    instruction = artifacts.get("instruction")
    classification_a = artifacts.get("classificationA")
    classification_b = artifacts.get("classificationB")
    policy = artifacts.get("policy")
    if not isinstance(instruction, dict):
        raise ValueError("artifacts.instruction must be an object")
    if not isinstance(classification_a, dict) or not isinstance(classification_b, dict):
        raise ValueError("artifacts.classificationA and artifacts.classificationB must be objects")
    if policy is not None and not isinstance(policy, dict):
        raise ValueError("artifacts.policy must be an object when provided")

    # Ensure envelope canonicalization is well-defined.
    compute_instruction_digest(instruction)

    left = canonical_instruction_classification(classification_a, "artifacts.classificationA")
    right = canonical_instruction_classification(classification_b, "artifacts.classificationB")
    if left != right:
        return VectorOutcome("rejected", "rejected", ["instruction_type_non_deterministic"])

    allow_unknown = False
    if isinstance(policy, dict):
        allow_unknown = bool(policy.get("allowUnknown", False))
    if left["state"] == "unknown" and not allow_unknown:
        return VectorOutcome("rejected", "rejected", ["instruction_unknown_unroutable"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_instruction_typing_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "instruction_typing" and CAPABILITY_INSTRUCTION_TYPING not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_instruction_typing_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_INSTRUCTION_TYPING not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_instruction_proposal_checking(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_INSTRUCTION_TYPING not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    proposal_a = artifacts.get("proposalA")
    proposal_b = artifacts.get("proposalB")
    classification_a = artifacts.get("classificationA")
    classification_b = artifacts.get("classificationB")
    policy = artifacts.get("policy")

    if not isinstance(proposal_a, dict) or not isinstance(proposal_b, dict):
        return VectorOutcome("rejected", "rejected", ["proposal_invalid_shape"])
    if not isinstance(classification_a, dict) or not isinstance(classification_b, dict):
        raise ValueError("artifacts.classificationA and artifacts.classificationB must be objects")
    if policy is not None and not isinstance(policy, dict):
        raise ValueError("artifacts.policy must be an object when provided")

    left = canonical_instruction_classification(classification_a, "artifacts.classificationA")
    right = canonical_instruction_classification(classification_b, "artifacts.classificationB")
    if left != right:
        return VectorOutcome("rejected", "rejected", ["instruction_type_non_deterministic"])

    allow_unknown = False
    if isinstance(policy, dict):
        allow_unknown = bool(policy.get("allowUnknown", False))
    if left["state"] == "unknown" and not allow_unknown:
        return VectorOutcome("rejected", "rejected", ["instruction_unknown_unroutable"])

    proposal_view_a, err_a = checked_llm_proposal(proposal_a)
    if err_a is not None:
        return VectorOutcome("rejected", "rejected", [err_a])
    proposal_view_b, err_b = checked_llm_proposal(proposal_b)
    if err_b is not None:
        return VectorOutcome("rejected", "rejected", [err_b])

    canonical_a = proposal_view_a["canonical"]
    canonical_b = proposal_view_b["canonical"]
    computed_a = proposal_view_a["digest"]
    computed_b = proposal_view_b["digest"]
    computed_kcir_a = proposal_view_a["kcirRef"]
    computed_kcir_b = proposal_view_b["kcirRef"]

    if canonical_a != canonical_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    if computed_a != computed_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    if computed_kcir_a != computed_kcir_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    obligations_a = proposal_view_a["obligations"]
    obligations_b = proposal_view_b["obligations"]
    if obligations_a != obligations_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    discharge_a = proposal_view_a["discharge"]
    discharge_b = proposal_view_b["discharge"]
    if discharge_a != discharge_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])
    if discharge_a.get("outcome") == "rejected":
        failure_classes = ensure_string_list(
            discharge_a.get("failureClasses", []),
            "artifacts.proposal.discharge.failureClasses",
        )
        if failure_classes:
            return VectorOutcome("rejected", "rejected", sorted(set(failure_classes)))
        return VectorOutcome("rejected", "rejected", ["descent_failure"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_instruction_typing_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/instruction_typed_deterministic":
        return evaluate_instruction_typed_deterministic(case)
    if vector_id == "golden/instruction_proposal_typed_deterministic":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/instruction_unknown_unroutable_reject":
        return evaluate_instruction_typed_deterministic(case)
    if vector_id == "adversarial/proposal_unbound_policy_reject":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/proposal_invalid_step_reject":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/proposal_nondeterministic_digest_reject":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/proposal_kcir_ref_mismatch_reject":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/proposal_ext_gap_discharge_reject":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/proposal_ext_ambiguous_discharge_reject":
        return evaluate_instruction_proposal_checking(case)
    if vector_id == "adversarial/instruction_typing_requires_claim":
        return evaluate_instruction_typing_requires_claim(case)
    if vector_id.startswith("invariance/"):
        return evaluate_instruction_typing_invariance(case)
    raise ValueError(f"unsupported instruction_typing vector id: {vector_id}")


def run_instruction_typing(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_instruction_typing_vector, errors)


def evaluate_adjoints_sites_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "adjoints_sites_obligations" and CAPABILITY_ADJOINTS_SITES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_adjoints_sites_proposal(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_ADJOINTS_SITES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    proposal_a = artifacts.get("proposalA")
    proposal_b = artifacts.get("proposalB")
    if not isinstance(proposal_a, dict) or not isinstance(proposal_b, dict):
        return VectorOutcome("rejected", "rejected", ["proposal_invalid_shape"])

    proposal_view_a, err_a = checked_llm_proposal(proposal_a)
    if err_a is not None:
        return VectorOutcome("rejected", "rejected", [err_a])
    proposal_view_b, err_b = checked_llm_proposal(proposal_b)
    if err_b is not None:
        return VectorOutcome("rejected", "rejected", [err_b])

    canonical_a = proposal_view_a["canonical"]
    canonical_b = proposal_view_b["canonical"]

    if canonical_a.get("proposalKind") != "refinementPlan" or canonical_b.get("proposalKind") != "refinementPlan":
        return VectorOutcome("rejected", "rejected", ["adjoints_sites_requires_refinement_plan"])

    if canonical_a != canonical_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    obligations_a = proposal_view_a["obligations"]
    obligations_b = proposal_view_b["obligations"]
    if obligations_a != obligations_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    obligation_kinds = {
        ensure_string(item.get("kind"), "obligation.kind")
        for item in obligations_a
        if isinstance(item, dict)
    }
    missing = sorted(ADJOINTS_SITES_REQUIRED_OBLIGATIONS - obligation_kinds)
    if missing:
        return VectorOutcome("rejected", "rejected", ["adjoints_sites_obligation_missing"])

    discharge_a = proposal_view_a["discharge"]
    discharge_b = proposal_view_b["discharge"]
    if discharge_a != discharge_b:
        return VectorOutcome("rejected", "rejected", ["proposal_nondeterministic"])

    outcome = ensure_string(discharge_a.get("outcome"), "discharge.outcome")
    failure_classes = canonical_check_set(discharge_a.get("failureClasses", []), "discharge.failureClasses")
    if outcome == "rejected":
        return VectorOutcome("rejected", "rejected", failure_classes or ["adjoint_triple_coherence_failure"])
    if outcome != "accepted":
        raise ValueError("discharge.outcome must be 'accepted' or 'rejected'")

    return VectorOutcome("accepted", "accepted", [])


def evaluate_adjoints_sites_cross_lane_contract(
    artifacts: Dict[str, Any],
    label_prefix: str = "artifacts",
) -> Optional[VectorOutcome]:
    return evaluate_cross_lane_span_square_contract(
        artifacts,
        required_capabilities=(CAPABILITY_ADJOINTS_SITES, CAPABILITY_SQUEAK_SITE),
        label_prefix=label_prefix,
    )


def evaluate_cross_lane_span_square_contract(
    artifacts: Dict[str, Any],
    *,
    required_capabilities: Sequence[str],
    label_prefix: str = "artifacts",
) -> Optional[VectorOutcome]:
    claimed = set(
        ensure_string_list(artifacts.get("claimedCapabilities", []), f"{label_prefix}.claimedCapabilities")
    )
    if not set(required_capabilities).issubset(claimed):
        return VectorOutcome("rejected", "rejected", ["cross_lane_capability_missing"])

    route_obj = artifacts.get("crossLaneRoute")
    if not isinstance(route_obj, dict):
        raise ValueError(f"{label_prefix}.crossLaneRoute must be an object")
    route = ensure_string(route_obj.get("pullbackBaseChange"), f"{label_prefix}.crossLaneRoute.pullbackBaseChange")
    if route != REQUIRED_CROSS_LANE_ROUTE:
        return VectorOutcome("rejected", "rejected", ["cross_lane_route_missing"])

    witness_obj = artifacts.get("spanSquareWitness")
    if not isinstance(witness_obj, dict):
        raise ValueError(f"{label_prefix}.spanSquareWitness must be an object")
    square_id = ensure_string(witness_obj.get("squareId"), f"{label_prefix}.spanSquareWitness.squareId")
    witness_route = ensure_string(witness_obj.get("route"), f"{label_prefix}.spanSquareWitness.route")
    witness_digest = ensure_string(witness_obj.get("digest"), f"{label_prefix}.spanSquareWitness.digest")
    expected_witness_digest = "sqw1_" + stable_hash({"squareId": square_id, "route": witness_route})
    if witness_route != REQUIRED_CROSS_LANE_ROUTE or witness_digest != expected_witness_digest:
        return VectorOutcome("rejected", "rejected", ["cross_lane_witness_mismatch"])

    location_descriptor = artifacts.get("locationDescriptor")
    if not isinstance(location_descriptor, dict):
        raise ValueError(f"{label_prefix}.locationDescriptor must be an object")
    expected_loc_ref = ensure_string(artifacts.get("expectedLocRef"), f"{label_prefix}.expectedLocRef")
    actual_loc_ref = compute_site_loc_ref(location_descriptor, f"{label_prefix}.locationDescriptor")
    if actual_loc_ref != expected_loc_ref:
        return VectorOutcome("rejected", "rejected", ["cross_lane_transport_mismatch"])

    return None


def evaluate_adjoints_sites_composed(case: Dict[str, Any]) -> VectorOutcome:
    base_outcome = evaluate_adjoints_sites_proposal(case)
    if base_outcome.result != "accepted":
        return base_outcome

    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    contract_outcome = evaluate_adjoints_sites_cross_lane_contract(artifacts)
    if contract_outcome is not None:
        return contract_outcome
    return VectorOutcome("accepted", "accepted", [])


def evaluate_adjoints_sites_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_ADJOINTS_SITES not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_adjoints_sites_composed_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    contract_outcome = evaluate_adjoints_sites_cross_lane_contract(artifacts)
    if contract_outcome is not None:
        return contract_outcome

    location_descriptor = artifacts.get("locationDescriptor")
    if not isinstance(location_descriptor, dict):
        raise ValueError("artifacts.locationDescriptor must be an object")
    runtime_profile = ensure_string(location_descriptor.get("runtimeProfile"), "artifacts.locationDescriptor.runtimeProfile")
    if profile != runtime_profile:
        return VectorOutcome("rejected", "rejected", ["cross_lane_profile_mismatch"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_adjoints_sites_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/adjoint_site_obligations_accept":
        return evaluate_adjoints_sites_proposal(case)
    if vector_id == "golden/composed_sigpi_squeak_span_accept":
        return evaluate_adjoints_sites_composed(case)
    if vector_id == "adversarial/adjoint_triangle_missing_reject":
        return evaluate_adjoints_sites_proposal(case)
    if vector_id == "adversarial/beck_chevalley_sigma_missing_reject":
        return evaluate_adjoints_sites_proposal(case)
    if vector_id == "adversarial/beck_chevalley_pi_missing_reject":
        return evaluate_adjoints_sites_proposal(case)
    if vector_id == "adversarial/refinement_invariance_missing_reject":
        return evaluate_adjoints_sites_proposal(case)
    if vector_id == "adversarial/composed_sigpi_squeak_span_route_missing_reject":
        return evaluate_adjoints_sites_composed(case)
    if vector_id == "adversarial/composed_sigpi_squeak_transport_ref_mismatch_reject":
        return evaluate_adjoints_sites_composed(case)
    if vector_id == "adversarial/adjoints_sites_requires_claim":
        return evaluate_adjoints_sites_requires_claim(case)
    if vector_id in {
        "invariance/same_composed_sigpi_squeak_span_local",
        "invariance/same_composed_sigpi_squeak_span_external",
    }:
        return evaluate_adjoints_sites_composed_invariance(case)
    if vector_id.startswith("invariance/"):
        return evaluate_adjoints_sites_invariance(case)
    raise ValueError(f"unsupported adjoints_sites vector id: {vector_id}")


def run_adjoints_sites(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_adjoints_sites_vector, errors)


def canonical_check_set(value: Any, label: str) -> List[str]:
    return sorted(set(ensure_string_list(value, label)))


def evaluate_ci_witness_deterministic(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_CI_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    instruction = artifacts.get("instruction")
    witness_a = artifacts.get("witnessA")
    witness_b = artifacts.get("witnessB")
    if not isinstance(instruction, dict):
        raise ValueError("artifacts.instruction must be an object")
    if not isinstance(witness_a, dict) or not isinstance(witness_b, dict):
        raise ValueError("artifacts.witnessA and artifacts.witnessB must be objects")

    expected_digest = compute_instruction_digest(instruction)
    a_digest = ensure_string(witness_a.get("instructionDigest"), "artifacts.witnessA.instructionDigest")
    b_digest = ensure_string(witness_b.get("instructionDigest"), "artifacts.witnessB.instructionDigest")
    if a_digest != expected_digest or b_digest != expected_digest:
        return VectorOutcome("rejected", "rejected", ["ci_instruction_digest_mismatch"])

    a_verdict = ensure_string(witness_a.get("verdictClass"), "artifacts.witnessA.verdictClass")
    b_verdict = ensure_string(witness_b.get("verdictClass"), "artifacts.witnessB.verdictClass")
    if a_verdict not in {"accepted", "rejected"} or b_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.witness*.verdictClass must be 'accepted' or 'rejected'")

    a_required = canonical_check_set(witness_a.get("requiredChecks", []), "artifacts.witnessA.requiredChecks")
    b_required = canonical_check_set(witness_b.get("requiredChecks", []), "artifacts.witnessB.requiredChecks")
    a_executed = canonical_check_set(witness_a.get("executedChecks", []), "artifacts.witnessA.executedChecks")
    b_executed = canonical_check_set(witness_b.get("executedChecks", []), "artifacts.witnessB.executedChecks")
    a_failures = canonical_check_set(witness_a.get("failureClasses", []), "artifacts.witnessA.failureClasses")
    b_failures = canonical_check_set(witness_b.get("failureClasses", []), "artifacts.witnessB.failureClasses")

    deterministic = (
        a_verdict == b_verdict and
        a_required == b_required and
        a_executed == b_executed and
        a_failures == b_failures
    )
    if deterministic:
        return VectorOutcome("accepted", a_verdict, a_failures)
    return VectorOutcome("rejected", "rejected", ["ci_witness_non_deterministic"])


def evaluate_ci_witness_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "instruction_witness_determinism" and CAPABILITY_CI_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_ci_witness_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_CI_WITNESSES not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_ci_boundary_authority_lineage(case: Dict[str, Any]) -> VectorOutcome:
    profile = case.get("profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    if profile is not None:
        profile_name = ensure_string(profile, "profile")
        if profile_name != "local":
            claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
            if CAPABILITY_CI_WITNESSES not in claimed:
                return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    kernel_verdict = "accepted"
    gate_failure_classes: List[str] = []
    input_data = artifacts.get("input")
    if input_data is not None:
        if not isinstance(input_data, dict):
            raise ValueError("artifacts.input must be an object when present")
        kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
        if kernel_verdict not in {"accepted", "rejected"}:
            raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
        gate_failure_classes = canonical_check_set(
            input_data.get("gateFailureClasses", []),
            "artifacts.input.gateFailureClasses",
        )

    failures: List[str] = []
    registry_raw = artifacts.get("obligationRegistry")
    if not isinstance(registry_raw, dict):
        raise ValueError("artifacts.obligationRegistry must be an object")
    registry_kind = ensure_string(
        registry_raw.get("registryKind"),
        "artifacts.obligationRegistry.registryKind",
    )
    mappings_raw = registry_raw.get("mappings")
    if not isinstance(mappings_raw, list):
        raise ValueError("artifacts.obligationRegistry.mappings must be a list")

    obligation_to_failure: Dict[str, str] = {}
    for idx, row in enumerate(mappings_raw):
        if not isinstance(row, dict):
            raise ValueError(f"artifacts.obligationRegistry.mappings[{idx}] must be an object")
        obligation_kind = ensure_string(
            row.get("obligationKind"),
            f"artifacts.obligationRegistry.mappings[{idx}].obligationKind",
        )
        failure_class = ensure_string(
            row.get("failureClass"),
            f"artifacts.obligationRegistry.mappings[{idx}].failureClass",
        )
        existing = obligation_to_failure.get(obligation_kind)
        if existing is not None and existing != failure_class:
            failures.append("boundary_authority_registry_mismatch")
            continue
        obligation_to_failure[obligation_kind] = failure_class

    if registry_kind != "premath.obligation_gate_registry.v1":
        failures.append("boundary_authority_registry_mismatch")

    proposal_raw = artifacts.get("proposal")
    if not isinstance(proposal_raw, dict):
        raise ValueError("artifacts.proposal must be an object")
    proposal_obligations_raw = proposal_raw.get("obligations")
    if not isinstance(proposal_obligations_raw, list):
        raise ValueError("artifacts.proposal.obligations must be a list")
    proposal_obligation_kinds: List[str] = []
    for idx, row in enumerate(proposal_obligations_raw):
        if not isinstance(row, dict):
            raise ValueError(f"artifacts.proposal.obligations[{idx}] must be an object")
        proposal_obligation_kinds.append(
            ensure_string(
                row.get("kind"),
                f"artifacts.proposal.obligations[{idx}].kind",
            )
        )
    proposal_obligation_kinds = sorted(set(proposal_obligation_kinds))

    proposal_discharge_raw = proposal_raw.get("discharge")
    if not isinstance(proposal_discharge_raw, dict):
        raise ValueError("artifacts.proposal.discharge must be an object")
    proposal_steps_raw = proposal_discharge_raw.get("steps")
    if not isinstance(proposal_steps_raw, list):
        raise ValueError("artifacts.proposal.discharge.steps must be a list")
    failed_obligation_kinds: List[str] = []
    for idx, row in enumerate(proposal_steps_raw):
        if not isinstance(row, dict):
            raise ValueError(f"artifacts.proposal.discharge.steps[{idx}] must be an object")
        kind = ensure_string(
            row.get("kind"),
            f"artifacts.proposal.discharge.steps[{idx}].kind",
        )
        status = ensure_string(
            row.get("status"),
            f"artifacts.proposal.discharge.steps[{idx}].status",
        )
        if status not in {"passed", "failed"}:
            raise ValueError(
                f"artifacts.proposal.discharge.steps[{idx}].status must be 'passed' or 'failed'"
            )
        if status == "failed":
            failed_obligation_kinds.append(kind)
            step_failure_class = row.get("failureClass")
            if step_failure_class is not None:
                if not isinstance(step_failure_class, str) or not step_failure_class:
                    raise ValueError(
                        f"artifacts.proposal.discharge.steps[{idx}].failureClass must be a non-empty string when present"
                    )
                mapped_failure_class = obligation_to_failure.get(kind)
                if mapped_failure_class is None or mapped_failure_class != step_failure_class:
                    failures.append("boundary_authority_registry_mismatch")

    expected_semantic_failure_classes: List[str] = []
    if "boundary_authority_registry_mismatch" not in failures:
        for kind in sorted(set(failed_obligation_kinds)):
            mapped_failure_class = obligation_to_failure.get(kind)
            if mapped_failure_class is None:
                failures.append("boundary_authority_registry_mismatch")
                break
            expected_semantic_failure_classes.append(mapped_failure_class)
    expected_semantic_failure_classes = sorted(set(expected_semantic_failure_classes))

    coherence_raw = artifacts.get("coherence")
    if not isinstance(coherence_raw, dict):
        raise ValueError("artifacts.coherence must be an object")
    coherence_registry_kind = ensure_string(
        coherence_raw.get("obligationRegistryKind"),
        "artifacts.coherence.obligationRegistryKind",
    )
    if coherence_registry_kind != registry_kind:
        failures.append("boundary_authority_registry_mismatch")
    coherence_bidir_obligations = set(
        canonical_check_set(
            coherence_raw.get("bidirCheckerObligations", []),
            "artifacts.coherence.bidirCheckerObligations",
        )
    )

    ci_witness_raw = artifacts.get("ciWitness")
    if not isinstance(ci_witness_raw, dict):
        raise ValueError("artifacts.ciWitness must be an object")
    ci_typed_core_projection_digest = ensure_string(
        ci_witness_raw.get("typedCoreProjectionDigest"),
        "artifacts.ciWitness.typedCoreProjectionDigest",
    )
    ci_authority_payload_digest = ensure_string(
        ci_witness_raw.get("authorityPayloadDigest"),
        "artifacts.ciWitness.authorityPayloadDigest",
    )
    ci_normalizer_id = ensure_string(
        ci_witness_raw.get("normalizerId"),
        "artifacts.ciWitness.normalizerId",
    )
    ci_policy_digest = ensure_string(
        ci_witness_raw.get("policyDigest"),
        "artifacts.ciWitness.policyDigest",
    )
    expected_ci_typed_core_projection_digest = compute_typed_core_projection_digest(
        ci_authority_payload_digest,
        ci_normalizer_id,
        ci_policy_digest,
    )
    if ci_typed_core_projection_digest != expected_ci_typed_core_projection_digest:
        failures.append("boundary_authority_lineage_mismatch")
    if ci_typed_core_projection_digest == ci_authority_payload_digest:
        failures.append("boundary_authority_lineage_mismatch")
    ci_projection_digest = ci_witness_raw.get("projectionDigest")
    if ci_projection_digest is not None:
        ci_projection_digest_value = ensure_string(
            ci_projection_digest,
            "artifacts.ciWitness.projectionDigest",
        )
        if ci_projection_digest_value != ci_authority_payload_digest:
            failures.append("boundary_authority_lineage_mismatch")
    ci_semantic_failure_classes = canonical_check_set(
        ci_witness_raw.get("semanticFailureClasses", []),
        "artifacts.ciWitness.semanticFailureClasses",
    )
    ci_operational_failure_classes = canonical_check_set(
        ci_witness_raw.get("operationalFailureClasses", []),
        "artifacts.ciWitness.operationalFailureClasses",
    )
    ci_failure_classes = canonical_check_set(
        ci_witness_raw.get("failureClasses", []),
        "artifacts.ciWitness.failureClasses",
    )

    doctrine_site_raw = artifacts.get("doctrineSite")
    if doctrine_site_raw is not None:
        if not isinstance(doctrine_site_raw, dict):
            raise ValueError("artifacts.doctrineSite must be an object when present")
        tracked_digest = ensure_string(
            doctrine_site_raw.get("trackedDigest"),
            "artifacts.doctrineSite.trackedDigest",
        )
        generated_digest = ensure_string(
            doctrine_site_raw.get("generatedDigest"),
            "artifacts.doctrineSite.generatedDigest",
        )
        if tracked_digest != generated_digest:
            failures.append("boundary_authority_stale_generated")

    proposal_ingest_raw = ci_witness_raw.get("proposalIngest")
    if proposal_ingest_raw is not None:
        if not isinstance(proposal_ingest_raw, dict):
            raise ValueError("artifacts.ciWitness.proposalIngest must be an object when present")
        proposal_ingest_obligations_raw = proposal_ingest_raw.get("obligations", [])
        if not isinstance(proposal_ingest_obligations_raw, list):
            raise ValueError("artifacts.ciWitness.proposalIngest.obligations must be a list")
        proposal_ingest_obligation_kinds: List[str] = []
        for idx, row in enumerate(proposal_ingest_obligations_raw):
            if not isinstance(row, dict):
                raise ValueError(
                    f"artifacts.ciWitness.proposalIngest.obligations[{idx}] must be an object"
                )
            proposal_ingest_obligation_kinds.append(
                ensure_string(
                    row.get("kind"),
                    f"artifacts.ciWitness.proposalIngest.obligations[{idx}].kind",
                )
            )
        if sorted(set(proposal_ingest_obligation_kinds)) != proposal_obligation_kinds:
            failures.append("boundary_authority_lineage_mismatch")

        proposal_ingest_discharge_raw = proposal_ingest_raw.get("discharge")
        if not isinstance(proposal_ingest_discharge_raw, dict):
            raise ValueError("artifacts.ciWitness.proposalIngest.discharge must be an object")
        proposal_ingest_failure_classes = canonical_check_set(
            proposal_ingest_discharge_raw.get("failureClasses", []),
            "artifacts.ciWitness.proposalIngest.discharge.failureClasses",
        )
        if (
            "boundary_authority_registry_mismatch" not in failures
            and proposal_ingest_failure_classes != expected_semantic_failure_classes
        ):
            failures.append("boundary_authority_lineage_mismatch")

    if "boundary_authority_registry_mismatch" not in failures:
        proposal_discharge_failure_classes = canonical_check_set(
            proposal_discharge_raw.get("failureClasses", []),
            "artifacts.proposal.discharge.failureClasses",
        )
        if proposal_discharge_failure_classes != expected_semantic_failure_classes:
            failures.append("boundary_authority_lineage_mismatch")
        if ci_semantic_failure_classes != expected_semantic_failure_classes:
            failures.append("boundary_authority_lineage_mismatch")
        if not set(proposal_obligation_kinds).issubset(coherence_bidir_obligations):
            failures.append("boundary_authority_lineage_mismatch")

    expected_ci_failure_classes = sorted(
        set(ci_operational_failure_classes + ci_semantic_failure_classes)
    )
    if ci_failure_classes != expected_ci_failure_classes:
        failures.append("boundary_authority_lineage_mismatch")

    failure_classes = sorted(set(failures))
    if failure_classes:
        return VectorOutcome("rejected", "rejected", failure_classes)
    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_ci_obstruction_roundtrip(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
    if CAPABILITY_CI_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    input_data = artifacts.get("input")
    if input_data is not None and not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object when present")
    kernel_verdict = "accepted"
    gate_failure_classes: List[str] = []
    if isinstance(input_data, dict):
        kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
        if kernel_verdict not in {"accepted", "rejected"}:
            raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
        gate_failure_classes = canonical_check_set(
            input_data.get("gateFailureClasses", []),
            "artifacts.input.gateFailureClasses",
        )

    roundtrip = artifacts.get("obstructionRoundtrip")
    if not isinstance(roundtrip, dict):
        raise ValueError("artifacts.obstructionRoundtrip must be an object")
    rows = roundtrip.get("rows")
    if not isinstance(rows, list) or not rows:
        raise ValueError("artifacts.obstructionRoundtrip.rows must be a non-empty list")

    failures: List[str] = []
    observed_families: List[str] = []
    observed_issue_tags: List[str] = []
    for idx, row in enumerate(rows):
        if not isinstance(row, dict):
            raise ValueError(f"artifacts.obstructionRoundtrip.rows[{idx}] must be an object")
        source_class = ensure_string(
            row.get("sourceClass"),
            f"artifacts.obstructionRoundtrip.rows[{idx}].sourceClass",
        )
        expected_constructor_raw = row.get("expectedConstructor")
        if not isinstance(expected_constructor_raw, dict):
            raise ValueError(
                f"artifacts.obstructionRoundtrip.rows[{idx}].expectedConstructor must be an object"
            )
        expected_family = ensure_string(
            expected_constructor_raw.get("family"),
            f"artifacts.obstructionRoundtrip.rows[{idx}].expectedConstructor.family",
        )
        expected_tag = ensure_string(
            expected_constructor_raw.get("tag"),
            f"artifacts.obstructionRoundtrip.rows[{idx}].expectedConstructor.tag",
        )
        expected_canonical = ensure_string(
            row.get("expectedCanonicalClass"),
            f"artifacts.obstructionRoundtrip.rows[{idx}].expectedCanonicalClass",
        )

        mapped = OBSTRUCTION_CLASS_TO_CONSTRUCTOR.get(source_class)
        if mapped is None:
            failures.append("obstruction_roundtrip_unknown_class")
            continue
        mapped_family, mapped_tag, mapped_canonical = mapped
        observed_families.append(mapped_family)
        observed_issue_tags.append(f"obs.{mapped_family}.{mapped_tag}")

        if (
            expected_family != mapped_family
            or expected_tag != mapped_tag
            or expected_canonical != mapped_canonical
        ):
            failures.append("obstruction_roundtrip_mismatch")
            continue

        roundtrip_canonical = OBSTRUCTION_CONSTRUCTOR_TO_CANONICAL.get((expected_family, expected_tag))
        if roundtrip_canonical != expected_canonical:
            failures.append("obstruction_roundtrip_mismatch")

    required_families = canonical_check_set(
        roundtrip.get("requiredFamilies", []),
        "artifacts.obstructionRoundtrip.requiredFamilies",
    )
    if required_families:
        missing_families = sorted(set(required_families) - set(observed_families))
        if missing_families:
            failures.append("obstruction_roundtrip_mismatch")

    issue_projection = roundtrip.get("issueProjection")
    if issue_projection is not None:
        if not isinstance(issue_projection, dict):
            raise ValueError("artifacts.obstructionRoundtrip.issueProjection must be an object")
        expected_tags = canonical_check_set(
            issue_projection.get("expectedTags", []),
            "artifacts.obstructionRoundtrip.issueProjection.expectedTags",
        )
        if expected_tags != canonical_check_set(observed_issue_tags, "observedIssueTags"):
            failures.append("obstruction_roundtrip_mismatch")

    failure_classes = sorted(set(failures))
    if failure_classes:
        return VectorOutcome("rejected", "rejected", failure_classes)
    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_ci_witness_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id in {
        "golden/instruction_witness_deterministic",
        "golden/instruction_reject_witness_deterministic",
        "adversarial/instruction_witness_non_deterministic_reject",
        "adversarial/instruction_reject_witness_failure_class_mismatch_reject",
    }:
        return evaluate_ci_witness_deterministic(case)
    if vector_id == "adversarial/instruction_witness_requires_claim":
        return evaluate_ci_witness_requires_claim(case)
    if vector_id in {
        "golden/witness_verifies_for_projected_delta",
        "golden/gate_witness_refs_integrity_accept",
        "golden/native_required_source_accept",
        "adversarial/witness_projection_digest_mismatch_reject",
        "adversarial/witness_verdict_inconsistent_reject",
        "adversarial/gate_witness_ref_digest_mismatch_reject",
        "adversarial/gate_witness_ref_source_missing_reject",
        "adversarial/native_required_fallback_reject",
    }:
        return evaluate_ci_required_witness_validity(case)
    if vector_id == "adversarial/ci_witness_requires_claim":
        return evaluate_ci_required_witness_requires_claim(case)
    if vector_id in {
        "golden/strict_delta_compare_match",
        "adversarial/strict_delta_compare_mismatch_reject",
    }:
        return evaluate_ci_required_witness_strict_delta(case)
    if vector_id in {
        "golden/decision_attestation_chain_accept",
        "adversarial/decision_attestation_witness_sha_mismatch_reject",
        "adversarial/decision_attestation_delta_sha_mismatch_reject",
    }:
        return evaluate_ci_required_witness_decision_attestation(case)
    if vector_id == "golden/delta_snapshot_projection_decision_stable":
        return evaluate_ci_required_witness_delta_snapshot(case)
    if vector_id in {
        "golden/boundary_authority_lineage_accept",
        "golden/obstruction_algebra_roundtrip_accept",
        "adversarial/boundary_authority_registry_mismatch_reject",
        "adversarial/boundary_authority_stale_generated_reject",
        "adversarial/obstruction_algebra_roundtrip_mismatch_reject",
        "invariance/same_boundary_authority_local",
        "invariance/same_boundary_authority_external",
    }:
        if vector_id in {
            "golden/obstruction_algebra_roundtrip_accept",
            "adversarial/obstruction_algebra_roundtrip_mismatch_reject",
        }:
            return evaluate_ci_obstruction_roundtrip(case)
        return evaluate_ci_boundary_authority_lineage(case)
    if vector_id in {
        "invariance/same_required_witness_local",
        "invariance/same_required_witness_external",
    }:
        return evaluate_ci_required_witness_invariance(case)
    if vector_id.startswith("invariance/"):
        return evaluate_ci_witness_invariance(case)
    raise ValueError(f"unsupported ci_witnesses vector id: {vector_id}")


def run_ci_witnesses(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_ci_witness_vector, errors)


def evaluate_change_projection_docs_and_code(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    expected_required = canonical_check_set(
        artifacts.get("expectedRequiredChecks", []), "artifacts.expectedRequiredChecks"
    )

    projection = project_required_checks(changed_paths)
    actual_required = sorted(projection.required_checks)
    if actual_required != expected_required:
        return VectorOutcome("rejected", "rejected", ["change_projection_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_provider_env_mapping(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    expected_required = canonical_check_set(
        artifacts.get("expectedRequiredChecks", []), "artifacts.expectedRequiredChecks"
    )
    direct_env = ensure_string_mapping(artifacts.get("directEnv"), "artifacts.directEnv")
    github_env = ensure_string_mapping(artifacts.get("githubEnv"), "artifacts.githubEnv")

    projection_direct = project_required_checks(changed_paths)
    projection_mapped = project_required_checks(changed_paths)
    actual_required = sorted(projection_direct.required_checks)
    if actual_required != expected_required:
        return VectorOutcome("rejected", "rejected", ["change_projection_mismatch"])
    if projection_direct.projection_digest != projection_mapped.projection_digest:
        return VectorOutcome("rejected", "rejected", ["change_projection_digest_mismatch"])

    mapped_env = map_github_to_premath_env(github_env)
    direct_refs = resolve_premath_ci_refs(direct_env)
    mapped_refs = resolve_premath_ci_refs(mapped_env)
    if direct_refs != mapped_refs:
        return VectorOutcome("rejected", "rejected", ["provider_env_mapping_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "change_morphisms" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_issue_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_claim" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    issue_before = artifacts.get("issueBefore")
    if not isinstance(issue_before, dict):
        raise ValueError("artifacts.issueBefore must be an object")
    issue_id = ensure_string(issue_before.get("id"), "artifacts.issueBefore.id")
    before_status = ensure_string(issue_before.get("status"), "artifacts.issueBefore.status")
    before_assignee_raw = issue_before.get("assignee", "")
    if not isinstance(before_assignee_raw, str):
        raise ValueError("artifacts.issueBefore.assignee must be a string when present")
    before_assignee = before_assignee_raw
    now_unix_ms = ensure_int(artifacts.get("nowUnixMs", 0), "artifacts.nowUnixMs")

    before_lease = issue_before.get("lease")
    before_lease_owner: Optional[str] = None
    before_lease_expires_at_unix_ms: Optional[int] = None
    if before_lease is not None:
        if not isinstance(before_lease, dict):
            raise ValueError("artifacts.issueBefore.lease must be an object when present")
        before_lease_owner = ensure_string(
            before_lease.get("owner"),
            "artifacts.issueBefore.lease.owner",
        )
        before_lease_expires_at_unix_ms = ensure_int(
            before_lease.get("expiresAtUnixMs"),
            "artifacts.issueBefore.lease.expiresAtUnixMs",
        )

    claim = artifacts.get("claim")
    if not isinstance(claim, dict):
        raise ValueError("artifacts.claim must be an object")
    claim_assignee = ensure_string(claim.get("assignee"), "artifacts.claim.assignee")
    claim_lease_id = claim.get("leaseId")
    if claim_lease_id is not None and not isinstance(claim_lease_id, str):
        raise ValueError("artifacts.claim.leaseId must be a string when present")
    claim_lease_ttl_seconds = claim.get("leaseTtlSeconds")
    claim_lease_expires_at_unix_ms = claim.get("leaseExpiresAtUnixMs")

    if before_status == "closed":
        return VectorOutcome("rejected", "rejected", ["issue_claim_closed"])
    has_stale_lease = (
        before_lease_owner is not None
        and before_lease_expires_at_unix_ms is not None
        and before_lease_expires_at_unix_ms <= now_unix_ms
    )
    has_active_lease = (
        before_lease_owner is not None
        and before_lease_expires_at_unix_ms is not None
        and before_lease_expires_at_unix_ms > now_unix_ms
    )
    if (
        has_active_lease
        and before_lease_owner is not None
        and before_lease_owner != claim_assignee
    ):
        return VectorOutcome("rejected", "rejected", ["lease_contention_active"])
    if before_assignee and before_assignee != claim_assignee and not has_active_lease and not has_stale_lease:
        return VectorOutcome("rejected", "rejected", ["issue_already_claimed"])

    lease_id = resolve_lease_id(claim_lease_id, issue_id, claim_assignee)
    lease_expires_at_unix_ms, expiry_error = resolve_lease_expiry_unix_ms(
        now_unix_ms,
        claim_lease_ttl_seconds,
        claim_lease_expires_at_unix_ms,
    )
    if expiry_error is not None:
        return VectorOutcome("rejected", "rejected", [expiry_error])
    if lease_expires_at_unix_ms is None:
        return VectorOutcome("rejected", "rejected", ["lease_invalid_expires_at"])

    actual_after = {
        "status": "in_progress",
        "assignee": claim_assignee,
        "lease": {
            "leaseId": lease_id,
            "owner": claim_assignee,
            "state": "active" if lease_expires_at_unix_ms > now_unix_ms else "stale",
        },
    }

    expected_after = artifacts.get("expectedAfter")
    if expected_after is not None:
        if not isinstance(expected_after, dict):
            raise ValueError("artifacts.expectedAfter must be an object when present")
        expected_status = ensure_string(expected_after.get("status"), "artifacts.expectedAfter.status")
        expected_assignee = ensure_string(
            expected_after.get("assignee"),
            "artifacts.expectedAfter.assignee",
        )
        expected_lease = expected_after.get("lease")
        if expected_lease is not None:
            if not isinstance(expected_lease, dict):
                raise ValueError("artifacts.expectedAfter.lease must be an object when present")
            expected_lease_id = ensure_string(
                expected_lease.get("leaseId"),
                "artifacts.expectedAfter.lease.leaseId",
            )
            expected_lease_owner = ensure_string(
                expected_lease.get("owner"),
                "artifacts.expectedAfter.lease.owner",
            )
            expected_lease_state = ensure_string(
                expected_lease.get("state"),
                "artifacts.expectedAfter.lease.state",
            )
            if (
                actual_after["lease"]["leaseId"] != expected_lease_id
                or actual_after["lease"]["owner"] != expected_lease_owner
                or actual_after["lease"]["state"] != expected_lease_state
            ):
                return VectorOutcome("rejected", "rejected", ["issue_claim_transition_mismatch"])
        if actual_after["status"] != expected_status or actual_after["assignee"] != expected_assignee:
            return VectorOutcome("rejected", "rejected", ["issue_claim_transition_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_issue_lease_renew(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_lease_renew" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    now_unix_ms = ensure_int(artifacts.get("nowUnixMs", 0), "artifacts.nowUnixMs")
    issue_before = artifacts.get("issueBefore")
    if not isinstance(issue_before, dict):
        raise ValueError("artifacts.issueBefore must be an object")
    before_status = ensure_string(issue_before.get("status"), "artifacts.issueBefore.status")

    before_lease = issue_before.get("lease")
    if before_lease is None:
        return VectorOutcome("rejected", "rejected", ["lease_missing"])
    if not isinstance(before_lease, dict):
        raise ValueError("artifacts.issueBefore.lease must be an object when present")
    before_lease_owner = ensure_string(
        before_lease.get("owner"),
        "artifacts.issueBefore.lease.owner",
    )
    before_lease_id = ensure_string(
        before_lease.get("leaseId"),
        "artifacts.issueBefore.lease.leaseId",
    )
    before_lease_expires_at_unix_ms = ensure_int(
        before_lease.get("expiresAtUnixMs"),
        "artifacts.issueBefore.lease.expiresAtUnixMs",
    )

    renew = artifacts.get("renew")
    if not isinstance(renew, dict):
        raise ValueError("artifacts.renew must be an object")
    renew_assignee = ensure_string(renew.get("assignee"), "artifacts.renew.assignee")
    renew_lease_id = ensure_string(renew.get("leaseId"), "artifacts.renew.leaseId")
    renew_lease_ttl_seconds = renew.get("leaseTtlSeconds")
    renew_lease_expires_at_unix_ms = renew.get("leaseExpiresAtUnixMs")
    lease_expires_at_unix_ms, expiry_error = resolve_lease_expiry_unix_ms(
        now_unix_ms,
        renew_lease_ttl_seconds,
        renew_lease_expires_at_unix_ms,
    )
    if expiry_error is not None:
        return VectorOutcome("rejected", "rejected", [expiry_error])
    if lease_expires_at_unix_ms is None:
        return VectorOutcome("rejected", "rejected", ["lease_invalid_expires_at"])

    if before_status == "closed":
        return VectorOutcome("rejected", "rejected", ["lease_issue_closed"])
    if before_lease_expires_at_unix_ms <= now_unix_ms:
        return VectorOutcome("rejected", "rejected", ["lease_stale"])
    if before_lease_owner != renew_assignee:
        return VectorOutcome("rejected", "rejected", ["lease_owner_mismatch"])
    if before_lease_id != renew_lease_id:
        return VectorOutcome("rejected", "rejected", ["lease_id_mismatch"])

    actual_after = {
        "status": "in_progress",
        "assignee": renew_assignee,
        "lease": {
            "leaseId": renew_lease_id,
            "owner": renew_assignee,
            "state": "active",
        },
    }

    expected_after = artifacts.get("expectedAfter")
    if expected_after is not None:
        if not isinstance(expected_after, dict):
            raise ValueError("artifacts.expectedAfter must be an object when present")
        expected_status = ensure_string(expected_after.get("status"), "artifacts.expectedAfter.status")
        expected_assignee = ensure_string(
            expected_after.get("assignee"),
            "artifacts.expectedAfter.assignee",
        )
        expected_lease = expected_after.get("lease")
        if not isinstance(expected_lease, dict):
            raise ValueError("artifacts.expectedAfter.lease must be an object")
        expected_lease_id = ensure_string(
            expected_lease.get("leaseId"),
            "artifacts.expectedAfter.lease.leaseId",
        )
        expected_lease_owner = ensure_string(
            expected_lease.get("owner"),
            "artifacts.expectedAfter.lease.owner",
        )
        expected_lease_state = ensure_string(
            expected_lease.get("state"),
            "artifacts.expectedAfter.lease.state",
        )
        if (
            actual_after["status"] != expected_status
            or actual_after["assignee"] != expected_assignee
            or actual_after["lease"]["leaseId"] != expected_lease_id
            or actual_after["lease"]["owner"] != expected_lease_owner
            or actual_after["lease"]["state"] != expected_lease_state
        ):
            return VectorOutcome("rejected", "rejected", ["issue_lease_renew_transition_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_issue_lease_release(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_lease_release" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    issue_before = artifacts.get("issueBefore")
    if not isinstance(issue_before, dict):
        raise ValueError("artifacts.issueBefore must be an object")
    before_status = ensure_string(issue_before.get("status"), "artifacts.issueBefore.status")
    before_assignee_raw = issue_before.get("assignee", "")
    if not isinstance(before_assignee_raw, str):
        raise ValueError("artifacts.issueBefore.assignee must be a string when present")
    before_assignee = before_assignee_raw

    release = artifacts.get("release")
    if not isinstance(release, dict):
        raise ValueError("artifacts.release must be an object")
    release_assignee = release.get("assignee")
    if release_assignee is not None and not isinstance(release_assignee, str):
        raise ValueError("artifacts.release.assignee must be a string when present")
    expected_assignee = release_assignee if isinstance(release_assignee, str) and release_assignee else None
    release_lease_id = release.get("leaseId")
    if release_lease_id is not None and not isinstance(release_lease_id, str):
        raise ValueError("artifacts.release.leaseId must be a string when present")
    expected_lease_id = release_lease_id if isinstance(release_lease_id, str) and release_lease_id else None

    before_lease = issue_before.get("lease")
    if before_lease is not None and not isinstance(before_lease, dict):
        raise ValueError("artifacts.issueBefore.lease must be an object when present")

    if before_lease is None:
        if expected_assignee is not None or expected_lease_id is not None:
            return VectorOutcome("rejected", "rejected", ["lease_missing"])
        actual_after = {"status": before_status, "assignee": before_assignee, "lease": None}
    else:
        before_lease_owner = ensure_string(
            before_lease.get("owner"),
            "artifacts.issueBefore.lease.owner",
        )
        before_lease_id = ensure_string(
            before_lease.get("leaseId"),
            "artifacts.issueBefore.lease.leaseId",
        )
        if expected_assignee is not None and before_lease_owner != expected_assignee:
            return VectorOutcome("rejected", "rejected", ["lease_owner_mismatch"])
        if expected_lease_id is not None and before_lease_id != expected_lease_id:
            return VectorOutcome("rejected", "rejected", ["lease_id_mismatch"])
        actual_after = {
            "status": "open" if before_status == "in_progress" else before_status,
            "assignee": "",
            "lease": None,
        }

    expected_after = artifacts.get("expectedAfter")
    if expected_after is not None:
        if not isinstance(expected_after, dict):
            raise ValueError("artifacts.expectedAfter must be an object when present")
        expected_status = ensure_string(expected_after.get("status"), "artifacts.expectedAfter.status")
        expected_assignee_after_raw = expected_after.get("assignee", "")
        if not isinstance(expected_assignee_after_raw, str):
            raise ValueError("artifacts.expectedAfter.assignee must be a string")
        expected_assignee_after = expected_assignee_after_raw
        expected_lease = expected_after.get("lease")
        if expected_lease is not None:
            raise ValueError("artifacts.expectedAfter.lease must be null for release checks")
        if actual_after["status"] != expected_status or actual_after["assignee"] != expected_assignee_after:
            return VectorOutcome("rejected", "rejected", ["issue_lease_release_transition_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_composed_issue_claim(case: Dict[str, Any]) -> VectorOutcome:
    base_outcome = evaluate_change_projection_issue_claim(case)
    if base_outcome.result != "accepted":
        return base_outcome

    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    contract_outcome = evaluate_cross_lane_span_square_contract(
        artifacts,
        required_capabilities=(
            CAPABILITY_CHANGE_MORPHISMS,
            CAPABILITY_ADJOINTS_SITES,
            CAPABILITY_SQUEAK_SITE,
        ),
    )
    if contract_outcome is not None:
        return contract_outcome
    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_composed_issue_lease_renew(case: Dict[str, Any]) -> VectorOutcome:
    base_outcome = evaluate_change_projection_issue_lease_renew(case)
    if base_outcome.result != "accepted":
        return base_outcome

    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    contract_outcome = evaluate_cross_lane_span_square_contract(
        artifacts,
        required_capabilities=(
            CAPABILITY_CHANGE_MORPHISMS,
            CAPABILITY_ADJOINTS_SITES,
            CAPABILITY_SQUEAK_SITE,
        ),
    )
    if contract_outcome is not None:
        return contract_outcome
    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_composed_invariance(
    case: Dict[str, Any],
    mutation_evaluator: Callable[[Dict[str, Any]], VectorOutcome],
) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = canonical_check_set(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    contract_outcome = evaluate_cross_lane_span_square_contract(
        artifacts,
        required_capabilities=(
            CAPABILITY_CHANGE_MORPHISMS,
            CAPABILITY_ADJOINTS_SITES,
            CAPABILITY_SQUEAK_SITE,
        ),
    )
    if contract_outcome is not None:
        return contract_outcome

    location_descriptor = artifacts.get("locationDescriptor")
    if not isinstance(location_descriptor, dict):
        raise ValueError("artifacts.locationDescriptor must be an object")
    runtime_profile = ensure_string(location_descriptor.get("runtimeProfile"), "artifacts.locationDescriptor.runtimeProfile")
    if profile != runtime_profile:
        return VectorOutcome("rejected", "rejected", ["cross_lane_profile_mismatch"])

    mutation_outcome = mutation_evaluator(case)
    mutation_failure_classes = sorted(set(mutation_outcome.gate_failure_classes))
    if mutation_outcome.kernel_verdict != kernel_verdict:
        return VectorOutcome("rejected", "rejected", ["cross_lane_kernel_verdict_mismatch"])
    if mutation_failure_classes != gate_failure_classes:
        return VectorOutcome("rejected", "rejected", ["cross_lane_gate_failure_class_mismatch"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_change_projection_composed_issue_claim_invariance(case: Dict[str, Any]) -> VectorOutcome:
    return evaluate_change_projection_composed_invariance(case, evaluate_change_projection_issue_claim)


def evaluate_change_projection_composed_issue_lease_renew_invariance(case: Dict[str, Any]) -> VectorOutcome:
    return evaluate_change_projection_composed_invariance(case, evaluate_change_projection_issue_lease_renew)


def _extract_issue_ids(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    ids: List[str] = []
    for idx, item in enumerate(value):
        if isinstance(item, str):
            issue_id = item
        elif isinstance(item, dict):
            issue_id = ensure_string(item.get("id"), f"{label}[{idx}].id")
        else:
            raise ValueError(f"{label}[{idx}] must be string or object")
        ids.append(issue_id)
    return ids


def evaluate_change_projection_issue_discover(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_discover" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    existing_ids = _extract_issue_ids(artifacts.get("existingIssues", []), "artifacts.existingIssues")
    parent_issue = artifacts.get("parentIssue")
    if not isinstance(parent_issue, dict):
        raise ValueError("artifacts.parentIssue must be an object")
    parent_id = ensure_string(parent_issue.get("id"), "artifacts.parentIssue.id")

    discovered_issue = artifacts.get("discoveredIssue")
    if not isinstance(discovered_issue, dict):
        raise ValueError("artifacts.discoveredIssue must be an object")
    discovered_id = ensure_string(discovered_issue.get("id"), "artifacts.discoveredIssue.id")

    if parent_id not in existing_ids:
        return VectorOutcome("rejected", "rejected", ["issue_discover_parent_missing"])
    if discovered_id in existing_ids:
        return VectorOutcome("rejected", "rejected", ["issue_discover_id_conflict"])

    expected_dependency = artifacts.get("expectedDependency")
    if expected_dependency is not None:
        if not isinstance(expected_dependency, dict):
            raise ValueError("artifacts.expectedDependency must be an object when present")
        expected_issue_id = ensure_string(
            expected_dependency.get("issueId"),
            "artifacts.expectedDependency.issueId",
        )
        expected_depends_on = ensure_string(
            expected_dependency.get("dependsOnId"),
            "artifacts.expectedDependency.dependsOnId",
        )
        expected_type = ensure_string(
            expected_dependency.get("type"),
            "artifacts.expectedDependency.type",
        )
        if (
            expected_issue_id != discovered_id
            or expected_depends_on != parent_id
            or expected_type != "discovered-from"
        ):
            return VectorOutcome("rejected", "rejected", ["issue_discover_link_mismatch"])

    expected_total = artifacts.get("expectedTotalIssues")
    if expected_total is not None:
        if not isinstance(expected_total, int):
            raise ValueError("artifacts.expectedTotalIssues must be an integer when present")
        if len(existing_ids) + 1 != expected_total:
            return VectorOutcome("rejected", "rejected", ["issue_discover_non_loss_violation"])

    return VectorOutcome("accepted", "accepted", [])


def _extract_issue_graph_rows(value: Any, label: str) -> List[Dict[str, Any]]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")

    out: List[Dict[str, Any]] = []
    for idx, item in enumerate(value):
        if not isinstance(item, dict):
            raise ValueError(f"{label}[{idx}] must be an object")

        issue_id = ensure_string(item.get("id"), f"{label}[{idx}].id")
        status = ensure_string(item.get("status"), f"{label}[{idx}].status")
        deps_raw = item.get("dependencies", [])
        if not isinstance(deps_raw, list):
            raise ValueError(f"{label}[{idx}].dependencies must be a list")

        deps: List[Dict[str, str]] = []
        for didx, dep in enumerate(deps_raw):
            if not isinstance(dep, dict):
                raise ValueError(f"{label}[{idx}].dependencies[{didx}] must be an object")
            depends_on_id = ensure_string(
                dep.get("dependsOnId"),
                f"{label}[{idx}].dependencies[{didx}].dependsOnId",
            )
            dep_type = ensure_string(dep.get("type"), f"{label}[{idx}].dependencies[{didx}].type")
            deps.append({"dependsOnId": depends_on_id, "type": dep_type})

        out.append({"id": issue_id, "status": status, "dependencies": deps})

    return out


def evaluate_change_projection_issue_ready_blocked(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_ready_blocked" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    rows = _extract_issue_graph_rows(artifacts.get("issues", []), "artifacts.issues")
    expected_ready = canonical_check_set(artifacts.get("expectedReadyIds", []), "artifacts.expectedReadyIds")
    expected_blocked = canonical_check_set(
        artifacts.get("expectedBlockedIds", []),
        "artifacts.expectedBlockedIds",
    )

    status_by_id: Dict[str, str] = {}
    for row in rows:
        issue_id = row["id"]
        if issue_id in status_by_id:
            raise ValueError(f"duplicate issue id in artifacts.issues: {issue_id}")
        status_by_id[issue_id] = row["status"]

    def has_unresolved_blocker(row: Dict[str, Any]) -> bool:
        for dep in row["dependencies"]:
            dep_type = dep["type"]
            if dep_type not in BLOCKING_DEP_TYPES:
                continue
            blocker_status = status_by_id.get(dep["dependsOnId"])
            if blocker_status != "closed":
                return True
        return False

    ready_ids: List[str] = []
    blocked_ids: List[str] = []
    for row in rows:
        unresolved = has_unresolved_blocker(row)
        if row["status"] == "open" and not unresolved:
            ready_ids.append(row["id"])
        if row["status"] != "closed" and unresolved:
            blocked_ids.append(row["id"])

    ready_ids = sorted(ready_ids)
    blocked_ids = sorted(blocked_ids)

    if ready_ids != expected_ready:
        return VectorOutcome("rejected", "rejected", ["issue_ready_set_mismatch"])
    if blocked_ids != expected_blocked:
        return VectorOutcome("rejected", "rejected", ["issue_blocked_set_mismatch"])

    if set(ready_ids) & set(blocked_ids):
        return VectorOutcome("rejected", "rejected", ["issue_ready_blocked_overlap"])

    open_ids = sorted(row["id"] for row in rows if row["status"] == "open")
    blocked_open_ids = sorted(issue_id for issue_id in blocked_ids if status_by_id.get(issue_id) == "open")
    if sorted(set(ready_ids + blocked_open_ids)) != open_ids:
        return VectorOutcome("rejected", "rejected", ["issue_ready_blocked_open_partition_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_issue_lease_projection(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_lease_projection" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    rows = artifacts.get("issues", [])
    if not isinstance(rows, list):
        raise ValueError("artifacts.issues must be a list")
    now_unix_ms = ensure_int(artifacts.get("nowUnixMs", 0), "artifacts.nowUnixMs")
    expected_stale = canonical_check_set(
        artifacts.get("expectedStaleIssueIds", []),
        "artifacts.expectedStaleIssueIds",
    )
    expected_contended = canonical_check_set(
        artifacts.get("expectedContendedIssueIds", []),
        "artifacts.expectedContendedIssueIds",
    )

    stale_issue_ids: List[str] = []
    contended_issue_ids: List[str] = []
    for idx, row in enumerate(rows):
        if not isinstance(row, dict):
            raise ValueError(f"artifacts.issues[{idx}] must be an object")
        issue_id = ensure_string(row.get("id"), f"artifacts.issues[{idx}].id")
        status = ensure_string(row.get("status"), f"artifacts.issues[{idx}].status")
        assignee_raw = row.get("assignee", "")
        if not isinstance(assignee_raw, str):
            raise ValueError(f"artifacts.issues[{idx}].assignee must be a string when present")
        lease = row.get("lease")
        if lease is None:
            continue
        if not isinstance(lease, dict):
            raise ValueError(f"artifacts.issues[{idx}].lease must be an object when present")

        lease_owner = ensure_string(lease.get("owner"), f"artifacts.issues[{idx}].lease.owner")
        lease_expires_at_unix_ms = ensure_int(
            lease.get("expiresAtUnixMs"),
            f"artifacts.issues[{idx}].lease.expiresAtUnixMs",
        )

        if lease_expires_at_unix_ms <= now_unix_ms:
            stale_issue_ids.append(issue_id)
            continue

        if status != "in_progress" or assignee_raw != lease_owner:
            contended_issue_ids.append(issue_id)

    stale_issue_ids = sorted(set(stale_issue_ids))
    contended_issue_ids = sorted(set(contended_issue_ids))

    if stale_issue_ids != expected_stale:
        return VectorOutcome("rejected", "rejected", ["lease_stale_set_mismatch"])
    if contended_issue_ids != expected_contended:
        return VectorOutcome("rejected", "rejected", ["lease_contended_set_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def compute_issue_event_stream_ref(events_payload: Any) -> str:
    return "ev1_" + stable_hash(events_payload)


def compute_issue_snapshot_ref(snapshot_payload: Any) -> str:
    return "iss1_" + stable_hash(snapshot_payload)


def evaluate_change_projection_issue_event_replay_cache(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")
    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "issue_event_replay_cache" and CAPABILITY_CHANGE_MORPHISMS not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    events_payload = artifacts.get("events")
    if not isinstance(events_payload, list):
        raise ValueError("artifacts.events must be a list")
    snapshot_payload = artifacts.get("snapshot")
    if not isinstance(snapshot_payload, dict):
        raise ValueError("artifacts.snapshot must be an object")

    declared_event_ref = ensure_string(artifacts.get("eventStreamRef"), "artifacts.eventStreamRef")
    declared_snapshot_ref = ensure_string(artifacts.get("snapshotRef"), "artifacts.snapshotRef")
    expected_event_ref = compute_issue_event_stream_ref(events_payload)
    expected_snapshot_ref = compute_issue_snapshot_ref(snapshot_payload)
    if declared_event_ref != expected_event_ref or declared_snapshot_ref != expected_snapshot_ref:
        return VectorOutcome("rejected", "rejected", ["issue_event_replay_ref_mismatch"])

    expected_cache_hit = artifacts.get("expectedCacheHit")
    if not isinstance(expected_cache_hit, bool):
        raise ValueError("artifacts.expectedCacheHit must be a boolean")

    cache_entry = artifacts.get("cacheEntry")
    actual_cache_hit = False
    if cache_entry is not None:
        if not isinstance(cache_entry, dict):
            raise ValueError("artifacts.cacheEntry must be an object when present")
        cache_event_ref = ensure_string(cache_entry.get("eventStreamRef"), "artifacts.cacheEntry.eventStreamRef")
        cache_snapshot_ref = ensure_string(cache_entry.get("snapshotRef"), "artifacts.cacheEntry.snapshotRef")
        actual_cache_hit = (
            cache_event_ref == declared_event_ref and cache_snapshot_ref == declared_snapshot_ref
        )

    if actual_cache_hit != expected_cache_hit:
        return VectorOutcome("rejected", "rejected", ["issue_event_replay_cache_hit_mismatch"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_change_projection_issue_event_replay_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_CHANGE_MORPHISMS not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    replay_outcome = evaluate_change_projection_issue_event_replay_cache(case)
    if replay_outcome.result != "accepted":
        return replay_outcome

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_change_projection_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    expected_required = canonical_check_set(
        artifacts.get("expectedRequiredChecks", []), "artifacts.expectedRequiredChecks"
    )

    projection = project_required_checks(changed_paths)
    actual_required = sorted(projection.required_checks)
    if actual_required != expected_required:
        return VectorOutcome("rejected", "rejected", ["change_projection_mismatch"])

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_CHANGE_MORPHISMS not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_change_projection_provider_wrapper_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    expected_required = canonical_check_set(
        artifacts.get("expectedRequiredChecks", []), "artifacts.expectedRequiredChecks"
    )
    projection = project_required_checks(changed_paths)
    actual_required = sorted(projection.required_checks)
    if actual_required != expected_required:
        return VectorOutcome("rejected", "rejected", ["change_projection_mismatch"])

    expected_refs_raw = artifacts.get("expectedRefs")
    if not isinstance(expected_refs_raw, dict):
        raise ValueError("artifacts.expectedRefs must be an object")
    expected_base_ref = expected_refs_raw.get("baseRef")
    if expected_base_ref is not None and not isinstance(expected_base_ref, str):
        raise ValueError("artifacts.expectedRefs.baseRef must be null or string")
    expected_head_ref = ensure_string(expected_refs_raw.get("headRef"), "artifacts.expectedRefs.headRef")
    expected_refs = (expected_base_ref, expected_head_ref)

    if profile == "local":
        local_env = ensure_string_mapping(artifacts.get("localEnv"), "artifacts.localEnv")
        actual_refs = resolve_premath_ci_refs(local_env)
    elif profile == "external":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_CHANGE_MORPHISMS not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
        github_env = ensure_string_mapping(artifacts.get("githubEnv"), "artifacts.githubEnv")
        mapped_env = map_github_to_premath_env(github_env)
        actual_refs = resolve_premath_ci_refs(mapped_env)
    else:
        raise ValueError("profile must be 'local' or 'external'")

    if actual_refs != expected_refs:
        return VectorOutcome("rejected", "rejected", ["provider_env_mapping_mismatch"])

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )
    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_change_projection_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id in {
        "golden/docs_only_raw_runs_conformance_check",
        "golden/kernel_touch_runs_build_test_and_toys",
        "golden/conformance_touch_runs_conformance_and_toys",
        "golden/fallback_unknown_surface_runs_baseline",
        "golden/mixed_known_unknown_surface_runs_baseline",
    }:
        return evaluate_change_projection_docs_and_code(case)
    if vector_id == "golden/issue_claim_sets_in_progress_and_assignee":
        return evaluate_change_projection_issue_claim(case)
    if vector_id == "golden/issue_claim_assigns_active_lease":
        return evaluate_change_projection_issue_claim(case)
    if vector_id == "golden/issue_claim_reclaims_stale_lease":
        return evaluate_change_projection_issue_claim(case)
    if vector_id == "golden/composed_issue_claim_sigpi_squeak_span_accept":
        return evaluate_change_projection_composed_issue_claim(case)
    if vector_id == "golden/issue_discover_preserves_existing_and_links_discovered_from":
        return evaluate_change_projection_issue_discover(case)
    if vector_id == "golden/issue_lease_renew_preserves_active_claim":
        return evaluate_change_projection_issue_lease_renew(case)
    if vector_id == "golden/composed_issue_lease_renew_sigpi_squeak_span_accept":
        return evaluate_change_projection_composed_issue_lease_renew(case)
    if vector_id == "golden/issue_lease_release_reopens_issue":
        return evaluate_change_projection_issue_lease_release(case)
    if vector_id == "golden/issue_ready_blocked_partition_coherent":
        return evaluate_change_projection_issue_ready_blocked(case)
    if vector_id == "golden/issue_lease_projection_stale_and_contended":
        return evaluate_change_projection_issue_lease_projection(case)
    if vector_id == "golden/issue_event_replay_cache_hit_stable":
        return evaluate_change_projection_issue_event_replay_cache(case)
    if vector_id == "golden/provider_env_mapping_github_equiv":
        return evaluate_change_projection_provider_env_mapping(case)
    if vector_id in {
        "invariance/same_provider_wrapper_local_env",
        "invariance/same_provider_wrapper_github_env",
    }:
        return evaluate_change_projection_provider_wrapper_invariance(case)
    if vector_id in {
        "invariance/same_composed_issue_claim_sigpi_squeak_span_local",
        "invariance/same_composed_issue_claim_sigpi_squeak_span_external",
    }:
        return evaluate_change_projection_composed_issue_claim_invariance(case)
    if vector_id in {
        "invariance/same_composed_issue_lease_renew_sigpi_squeak_span_local",
        "invariance/same_composed_issue_lease_renew_sigpi_squeak_span_external",
    }:
        return evaluate_change_projection_composed_issue_lease_renew_invariance(case)
    if vector_id in {
        "invariance/same_issue_claim_contention_local",
        "invariance/same_issue_claim_contention_external",
    }:
        return evaluate_change_projection_issue_claim(case)
    if vector_id in {
        "invariance/same_issue_lease_renew_stale_local",
        "invariance/same_issue_lease_renew_stale_external",
    }:
        return evaluate_change_projection_issue_lease_renew(case)
    if vector_id in {
        "invariance/same_issue_lease_release_owner_mismatch_local",
        "invariance/same_issue_lease_release_owner_mismatch_external",
    }:
        return evaluate_change_projection_issue_lease_release(case)
    if vector_id == "adversarial/change_morphisms_requires_claim":
        return evaluate_change_projection_requires_claim(case)
    if vector_id == "adversarial/issue_ready_blocked_partition_mismatch_reject":
        return evaluate_change_projection_issue_ready_blocked(case)
    if vector_id == "adversarial/issue_ready_blocked_set_mismatch_reject":
        return evaluate_change_projection_issue_ready_blocked(case)
    if vector_id == "adversarial/issue_discover_rejects_parent_missing":
        return evaluate_change_projection_issue_discover(case)
    if vector_id == "adversarial/issue_claim_rejects_active_lease_contention":
        return evaluate_change_projection_issue_claim(case)
    if vector_id == "adversarial/composed_issue_claim_cross_lane_capability_missing_reject":
        return evaluate_change_projection_composed_issue_claim(case)
    if vector_id == "adversarial/composed_issue_claim_span_route_missing_reject":
        return evaluate_change_projection_composed_issue_claim(case)
    if vector_id == "adversarial/issue_claim_invalid_expiry_reject":
        return evaluate_change_projection_issue_claim(case)
    if vector_id == "adversarial/issue_claim_invalid_ttl_reject":
        return evaluate_change_projection_issue_claim(case)
    if vector_id == "adversarial/issue_lease_renew_stale_reject":
        return evaluate_change_projection_issue_lease_renew(case)
    if vector_id == "adversarial/composed_issue_lease_renew_transport_ref_mismatch_reject":
        return evaluate_change_projection_composed_issue_lease_renew(case)
    if vector_id == "adversarial/issue_lease_release_owner_mismatch_reject":
        return evaluate_change_projection_issue_lease_release(case)
    if vector_id == "adversarial/issue_lease_release_id_mismatch_reject":
        return evaluate_change_projection_issue_lease_release(case)
    if vector_id == "adversarial/issue_lease_projection_mismatch_reject":
        return evaluate_change_projection_issue_lease_projection(case)
    if vector_id == "adversarial/issue_event_replay_cache_ref_mismatch_reject":
        return evaluate_change_projection_issue_event_replay_cache(case)
    if vector_id in {
        "invariance/same_issue_event_replay_cache_local",
        "invariance/same_issue_event_replay_cache_external",
    }:
        return evaluate_change_projection_issue_event_replay_invariance(case)
    if vector_id.startswith("invariance/"):
        return evaluate_change_projection_invariance(case)
    raise ValueError(f"unsupported change_morphisms vector id: {vector_id}")


def run_change_projection(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_change_projection_vector, errors)


def evaluate_ci_required_witness_validity(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    witness = artifacts.get("witness")
    if not isinstance(witness, dict):
        raise ValueError("artifacts.witness must be an object")
    gate_witness_payloads = ensure_gate_witness_payloads(
        artifacts.get("gateWitnessPayloads"),
        "artifacts.gateWitnessPayloads",
    )
    native_required_checks = ensure_string_list(
        artifacts.get("nativeRequiredChecks", []),
        "artifacts.nativeRequiredChecks",
    )

    errors, _derived = verify_required_witness_payload(
        witness,
        changed_paths,
        gate_witness_payloads=gate_witness_payloads,
        native_required_checks=native_required_checks,
    )
    if errors:
        return VectorOutcome("rejected", "rejected", ["ci_required_witness_invalid"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_ci_required_witness_requires_claim(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    request = artifacts.get("request")
    if not isinstance(request, dict):
        raise ValueError("artifacts.request must be an object")

    mode = ensure_string(request.get("mode"), "artifacts.request.mode")
    claimed = set(
        ensure_string_list(request.get("claimedCapabilities", []), "artifacts.request.claimedCapabilities")
    )
    if mode == "ci_witness_verification" and CAPABILITY_CI_WITNESSES not in claimed:
        return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_ci_required_witness_invariance(case: Dict[str, Any]) -> VectorOutcome:
    profile = ensure_string(case.get("profile"), "profile")
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    input_data = artifacts.get("input")
    if not isinstance(input_data, dict):
        raise ValueError("artifacts.input must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    witness = artifacts.get("witness")
    if not isinstance(witness, dict):
        raise ValueError("artifacts.witness must be an object")
    gate_witness_payloads = ensure_gate_witness_payloads(
        artifacts.get("gateWitnessPayloads"),
        "artifacts.gateWitnessPayloads",
    )
    native_required_checks = ensure_string_list(
        artifacts.get("nativeRequiredChecks", []),
        "artifacts.nativeRequiredChecks",
    )

    verify_errors, _derived = verify_required_witness_payload(
        witness,
        changed_paths,
        gate_witness_payloads=gate_witness_payloads,
        native_required_checks=native_required_checks,
    )
    if verify_errors:
        return VectorOutcome("rejected", "rejected", ["ci_required_witness_invalid"])

    kernel_verdict = ensure_string(input_data.get("kernelVerdict"), "artifacts.input.kernelVerdict")
    if kernel_verdict not in {"accepted", "rejected"}:
        raise ValueError("artifacts.input.kernelVerdict must be 'accepted' or 'rejected'")
    gate_failure_classes = ensure_string_list(
        input_data.get("gateFailureClasses", []), "artifacts.input.gateFailureClasses"
    )

    if profile != "local":
        claimed = set(ensure_string_list(artifacts.get("claimedCapabilities", []), "claimedCapabilities"))
        if CAPABILITY_CI_WITNESSES not in claimed:
            return VectorOutcome("rejected", "rejected", ["capability_not_claimed"])

    return VectorOutcome(kernel_verdict, kernel_verdict, gate_failure_classes)


def evaluate_ci_required_witness_strict_delta(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    detected_paths = ensure_string_list(
        artifacts.get("detectedChangedPaths", []), "artifacts.detectedChangedPaths"
    )
    witness = artifacts.get("witness")
    if not isinstance(witness, dict):
        raise ValueError("artifacts.witness must be an object")
    gate_witness_payloads = ensure_gate_witness_payloads(
        artifacts.get("gateWitnessPayloads"),
        "artifacts.gateWitnessPayloads",
    )
    native_required_checks = ensure_string_list(
        artifacts.get("nativeRequiredChecks", []),
        "artifacts.nativeRequiredChecks",
    )

    verify_errors, _derived = verify_required_witness_payload(
        witness,
        changed_paths,
        gate_witness_payloads=gate_witness_payloads,
        native_required_checks=native_required_checks,
    )
    if verify_errors:
        return VectorOutcome("rejected", "rejected", ["ci_required_witness_invalid"])

    witness_paths = ensure_string_list(witness.get("changedPaths", []), "artifacts.witness.changedPaths")
    if sorted(set(witness_paths)) != sorted(set(detected_paths)):
        return VectorOutcome("rejected", "rejected", ["delta_comparison_mismatch"])
    return VectorOutcome("accepted", "accepted", [])


def evaluate_ci_required_witness_delta_snapshot(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    delta_snapshot = artifacts.get("deltaSnapshot")
    if not isinstance(delta_snapshot, dict):
        raise ValueError("artifacts.deltaSnapshot must be an object")
    snapshot_paths = ensure_string_list(
        delta_snapshot.get("changedPaths", []), "artifacts.deltaSnapshot.changedPaths"
    )

    witness = artifacts.get("witness")
    if not isinstance(witness, dict):
        raise ValueError("artifacts.witness must be an object")
    verify_errors, _derived = verify_required_witness_payload(
        witness,
        changed_paths,
    )
    if verify_errors:
        return VectorOutcome("rejected", "rejected", ["ci_required_witness_invalid"])

    projection = project_required_checks(snapshot_paths)
    witness_projection = ensure_string(witness.get("projectionDigest"), "artifacts.witness.projectionDigest")
    if projection.projection_digest != witness_projection:
        return VectorOutcome("rejected", "rejected", ["delta_snapshot_projection_mismatch"])

    if sorted(set(snapshot_paths)) != sorted(set(changed_paths)):
        return VectorOutcome("rejected", "rejected", ["delta_snapshot_paths_mismatch"])

    decision_snapshot = artifacts.get("decisionFromSnapshot")
    decision_detect = artifacts.get("decisionFromDetect")
    if not isinstance(decision_snapshot, dict) or not isinstance(decision_detect, dict):
        raise ValueError("artifacts.decisionFromSnapshot and artifacts.decisionFromDetect must be objects")

    def _decision_shape(decision: Dict[str, Any], label: str) -> Dict[str, Any]:
        return {
            "decision": ensure_string(decision.get("decision"), f"{label}.decision"),
            "projectionDigest": ensure_string(decision.get("projectionDigest"), f"{label}.projectionDigest"),
            "reasonClass": ensure_string(decision.get("reasonClass"), f"{label}.reasonClass"),
            "requiredChecks": canonical_check_set(
                decision.get("requiredChecks", []), f"{label}.requiredChecks"
            ),
        }

    snapshot_shape = _decision_shape(decision_snapshot, "artifacts.decisionFromSnapshot")
    detect_shape = _decision_shape(decision_detect, "artifacts.decisionFromDetect")
    if snapshot_shape != detect_shape:
        return VectorOutcome("rejected", "rejected", ["decision_non_deterministic"])
    if snapshot_shape["projectionDigest"] != projection.projection_digest:
        return VectorOutcome("rejected", "rejected", ["decision_projection_mismatch"])
    if snapshot_shape["decision"] != "accept":
        return VectorOutcome("rejected", "rejected", ["decision_not_accept"])

    return VectorOutcome("accepted", "accepted", [])


def evaluate_ci_required_witness_decision_attestation(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    changed_paths = ensure_string_list(artifacts.get("changedPaths", []), "artifacts.changedPaths")
    witness = artifacts.get("witness")
    delta_snapshot = artifacts.get("deltaSnapshot")
    decision = artifacts.get("decision")
    if not isinstance(witness, dict):
        raise ValueError("artifacts.witness must be an object")
    if not isinstance(delta_snapshot, dict):
        raise ValueError("artifacts.deltaSnapshot must be an object")
    if not isinstance(decision, dict):
        raise ValueError("artifacts.decision must be an object")

    verify_errors, _derived = verify_required_witness_payload(witness, changed_paths)
    if verify_errors:
        return VectorOutcome("rejected", "rejected", ["ci_required_witness_invalid"])

    projection = project_required_checks(changed_paths)
    expected_checks = projection.required_checks
    decision_checks = canonical_check_set(decision.get("requiredChecks", []), "artifacts.decision.requiredChecks")

    if witness.get("projectionDigest") != projection.projection_digest:
        return VectorOutcome("rejected", "rejected", ["decision_projection_mismatch"])
    if delta_snapshot.get("projectionDigest") != projection.projection_digest:
        return VectorOutcome("rejected", "rejected", ["decision_projection_mismatch"])
    if decision.get("projectionDigest") != projection.projection_digest:
        return VectorOutcome("rejected", "rejected", ["decision_projection_mismatch"])
    if decision_checks != expected_checks:
        return VectorOutcome("rejected", "rejected", ["decision_required_checks_mismatch"])

    if decision.get("decisionKind") != "ci.required.decision.v1":
        return VectorOutcome("rejected", "rejected", ["decision_kind_mismatch"])

    witness_sha = stable_hash(witness)
    delta_sha = stable_hash(delta_snapshot)
    if decision.get("witnessSha256") != witness_sha:
        return VectorOutcome("rejected", "rejected", ["decision_witness_sha_mismatch"])
    if decision.get("deltaSha256") != delta_sha:
        return VectorOutcome("rejected", "rejected", ["decision_delta_sha_mismatch"])

    decision_value = ensure_string(decision.get("decision"), "artifacts.decision.decision")
    if decision_value != "accept":
        return VectorOutcome("rejected", "rejected", ["decision_not_accept"])
    if decision.get("reasonClass") != "verified_accept":
        return VectorOutcome("rejected", "rejected", ["decision_reason_mismatch"])

    errors = decision.get("errors")
    if not isinstance(errors, list) or errors:
        return VectorOutcome("rejected", "rejected", ["decision_errors_non_empty"])

    return VectorOutcome("accepted", "accepted", [])


def parse_args() -> argparse.Namespace:
    default_fixtures = ROOT / "tests" / "conformance" / "fixtures" / "capabilities"
    parser = argparse.ArgumentParser(description="Run executable capability conformance vectors.")
    parser.add_argument(
        "--registry",
        type=Path,
        default=CAPABILITY_REGISTRY_PATH,
        help=f"Path to capability registry artifact (default: {CAPABILITY_REGISTRY_PATH})",
    )
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=default_fixtures,
        help=f"Path to capability fixture root (default: {default_fixtures})",
    )
    parser.add_argument(
        "--capability",
        action="append",
        default=None,
        help="Capability ID to run. Repeatable. Default: executableCapabilities from registry.",
    )
    return parser.parse_args()


CapabilityRunner = Callable[[Path, List[str]], Tuple[int, int]]


def capability_runners() -> Dict[str, CapabilityRunner]:
    return {
        CAPABILITY_NORMAL_FORMS: run_normal_forms,
        CAPABILITY_KCIR_WITNESSES: run_kcir_witnesses,
        CAPABILITY_COMMITMENT_CHECKPOINTS: run_commitment_checkpoints,
        CAPABILITY_SQUEAK_SITE: run_squeak_site,
        CAPABILITY_CI_WITNESSES: run_ci_witnesses,
        CAPABILITY_INSTRUCTION_TYPING: run_instruction_typing,
        CAPABILITY_ADJOINTS_SITES: run_adjoints_sites,
        CAPABILITY_CHANGE_MORPHISMS: run_change_projection,
    }


def main() -> int:
    args = parse_args()
    registry_path = args.registry
    fixtures_root = args.fixtures

    try:
        executable_capabilities = load_executable_capabilities(registry_path)
    except ValueError as err:
        print(f"[error] {err}")
        return 2

    runners = capability_runners()
    runner_capability_ids = set(runners.keys())
    executable_capability_ids = set(executable_capabilities)
    missing_runners = sorted(executable_capability_ids - runner_capability_ids)
    undeclared_runners = sorted(runner_capability_ids - executable_capability_ids)
    if missing_runners:
        print(f"[error] capability registry contains unsupported capability handlers: {missing_runners}")
        return 2
    if undeclared_runners:
        print(
            "[error] capability runner table contains undeclared executable capabilities: "
            f"{undeclared_runners}"
        )
        return 2

    capability_ids: Sequence[str] = args.capability or list(executable_capabilities)

    if not fixtures_root.exists():
        print(f"[error] fixtures path does not exist: {fixtures_root}")
        return 2
    if not fixtures_root.is_dir():
        print(f"[error] fixtures path is not a directory: {fixtures_root}")
        return 2

    errors: List[str] = []
    capability_count = 0
    checked_vectors = 0

    for capability_id in capability_ids:
        capability_dir = fixtures_root / capability_id
        if not capability_dir.exists():
            errors.append(f"missing capability directory: {capability_dir}")
            continue
        runner = runners.get(capability_id)
        if runner is None:
            errors.append(f"unsupported executable capability: {capability_id}")
            continue
        count, checked = runner(capability_dir, errors)
        capability_count += count
        checked_vectors += checked

    if errors:
        print(f"[conformance-run] FAIL ({len(errors)} errors)")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        f"[conformance-run] OK "
        f"(capabilities={capability_count}, vectors={checked_vectors})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
