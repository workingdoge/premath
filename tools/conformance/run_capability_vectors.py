#!/usr/bin/env python3
"""
Execute capability conformance vectors.

Current executable capability:
- capabilities.normal_forms
- capabilities.kcir_witnesses
- capabilities.commitment_checkpoints
- capabilities.squeak_site
- capabilities.ci_witnesses
- capabilities.instruction_typing
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

CAPABILITY_NORMAL_FORMS = "capabilities.normal_forms"
CAPABILITY_KCIR_WITNESSES = "capabilities.kcir_witnesses"
CAPABILITY_COMMITMENT_CHECKPOINTS = "capabilities.commitment_checkpoints"
CAPABILITY_SQUEAK_SITE = "capabilities.squeak_site"
CAPABILITY_CI_WITNESSES = "capabilities.ci_witnesses"
CAPABILITY_INSTRUCTION_TYPING = "capabilities.instruction_typing"
DEFAULT_EXECUTABLE_CAPABILITIES: Sequence[str] = (
    CAPABILITY_NORMAL_FORMS,
    CAPABILITY_KCIR_WITNESSES,
    CAPABILITY_COMMITMENT_CHECKPOINTS,
    CAPABILITY_SQUEAK_SITE,
    CAPABILITY_CI_WITNESSES,
    CAPABILITY_INSTRUCTION_TYPING,
)


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


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str):
            raise ValueError(f"{label}[{idx}] must be a string")
        out.append(item)
    return out


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
    intent = ensure_string(instruction.get("intent"), "artifacts.instruction.intent")
    if "scope" not in instruction:
        raise ValueError("artifacts.instruction.scope is required")
    scope = instruction.get("scope")
    if scope in (None, ""):
        raise ValueError("artifacts.instruction.scope must be non-empty")
    policy_digest = ensure_string(instruction.get("policyDigest"), "artifacts.instruction.policyDigest")
    requested_checks = ensure_string_list(
        instruction.get("requestedChecks", []), "artifacts.instruction.requestedChecks"
    )
    if len(set(requested_checks)) != len(requested_checks):
        raise ValueError("artifacts.instruction.requestedChecks must not contain duplicates")
    return {
        "intent": intent,
        "scope": scope,
        "policyDigest": policy_digest,
        "requestedChecks": requested_checks,
    }


def compute_instruction_digest(instruction: Dict[str, Any]) -> str:
    return "instr1_" + stable_hash(canonical_instruction_envelope(instruction))


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


def evaluate_instruction_typing_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/instruction_typed_deterministic":
        return evaluate_instruction_typed_deterministic(case)
    if vector_id == "adversarial/instruction_unknown_unroutable_reject":
        return evaluate_instruction_typed_deterministic(case)
    if vector_id == "adversarial/instruction_typing_requires_claim":
        return evaluate_instruction_typing_requires_claim(case)
    if vector_id.startswith("invariance/"):
        return evaluate_instruction_typing_invariance(case)
    raise ValueError(f"unsupported instruction_typing vector id: {vector_id}")


def run_instruction_typing(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_instruction_typing_vector, errors)


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

    deterministic = (
        a_verdict == b_verdict and
        a_required == b_required and
        a_executed == b_executed
    )
    if deterministic:
        return VectorOutcome("accepted", a_verdict, [])
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


def evaluate_ci_witness_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    if vector_id == "golden/instruction_witness_deterministic":
        return evaluate_ci_witness_deterministic(case)
    if vector_id == "adversarial/instruction_witness_non_deterministic_reject":
        return evaluate_ci_witness_deterministic(case)
    if vector_id == "adversarial/instruction_witness_requires_claim":
        return evaluate_ci_witness_requires_claim(case)
    if vector_id.startswith("invariance/"):
        return evaluate_ci_witness_invariance(case)
    raise ValueError(f"unsupported ci_witnesses vector id: {vector_id}")


def run_ci_witnesses(capability_dir: Path, errors: List[str]) -> Tuple[int, int]:
    return run_capability_vectors(capability_dir, evaluate_ci_witness_vector, errors)


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    default_fixtures = root / "tests" / "conformance" / "fixtures" / "capabilities"
    parser = argparse.ArgumentParser(description="Run executable capability conformance vectors.")
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
        help=(
            "Capability ID to run. Repeatable. "
            f"Default: {', '.join(DEFAULT_EXECUTABLE_CAPABILITIES)}"
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    fixtures_root = args.fixtures
    capability_ids: Sequence[str] = args.capability or list(DEFAULT_EXECUTABLE_CAPABILITIES)

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
        if capability_id == CAPABILITY_NORMAL_FORMS:
            count, checked = run_normal_forms(capability_dir, errors)
        elif capability_id == CAPABILITY_KCIR_WITNESSES:
            count, checked = run_kcir_witnesses(capability_dir, errors)
        elif capability_id == CAPABILITY_COMMITMENT_CHECKPOINTS:
            count, checked = run_commitment_checkpoints(capability_dir, errors)
        elif capability_id == CAPABILITY_SQUEAK_SITE:
            count, checked = run_squeak_site(capability_dir, errors)
        elif capability_id == CAPABILITY_CI_WITNESSES:
            count, checked = run_ci_witnesses(capability_dir, errors)
        elif capability_id == CAPABILITY_INSTRUCTION_TYPING:
            count, checked = run_instruction_typing(capability_dir, errors)
        else:
            errors.append(f"unsupported executable capability: {capability_id}")
            continue
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
