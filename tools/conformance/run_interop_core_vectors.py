#!/usr/bin/env python3
"""
Execute Interop Core conformance vectors.

Current slices cover deterministic checks for:
- draft/KCIR-CORE
- draft/REF-BINDING
- draft/NF
- draft/WIRE-FORMATS
- draft/ERROR-CODES
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "interop-core"

REQUIRED_DOMAINS = {"kcir.node", "kcir.obj_nf", "kcir.mor_nf"}
SUPPORTED_WIRE_FORMAT_IDS = {
    "kcir.wire.lenprefixed-ref.v1",
    "kcir.wire.legacy-fixed32.v1",
}
KNOWN_ERROR_CODES = {
    "kcir_v2.parse_error",
    "kcir_v2.env_uid_mismatch",
    "kcir_v2.dep_cycle",
    "kcir_v2.unsupported_sort",
    "kcir_v2.unsupported_opcode",
    "kcir_v2.contract_violation",
    "kcir_v2.profile_mismatch",
    "kcir_v2.params_hash_mismatch",
    "kcir_v2.domain_mismatch",
    "kcir_v2.digest_mismatch",
    "kcir_v2.evidence_malformed",
    "kcir_v2.evidence_invalid",
    "kcir_v2.anchor_mismatch",
    "kcir_v2.anchor_missing",
    "kcir_v2.store_missing_node",
    "kcir_v2.store_missing_obj_nf",
    "kcir_v2.store_missing_mor_nf",
    "kcir_v2.data_unavailable",
    "kcir_v2.obj_nf_noncanonical",
    "kcir_v2.mor_nf_noncanonical",
}

DEFAULT_REF_PROFILE_PATH = ROOT / "policies" / "ref" / "sha256_detached_v1.json"


@dataclass(frozen=True)
class VectorOutcome:
    result: str
    failure_classes: List[str]


class VectorError(Exception):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def resolve_premath_cli() -> List[str]:
    premath_bin = ROOT / "target" / "debug" / "premath"
    if premath_bin.exists() and os.access(premath_bin, os.X_OK):
        return [str(premath_bin)]
    return ["cargo", "run", "--package", "premath-cli", "--"]


def run_premath_json(args: Sequence[str]) -> Dict[str, Any]:
    cmd = [*resolve_premath_cli(), *args]
    completed = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise ValueError(
            "premath command failed: "
            f"{' '.join(cmd)}\n"
            f"stderr:\n{completed.stderr}\n"
            f"stdout:\n{completed.stdout}"
        )
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise ValueError(
            "premath command returned invalid JSON\n"
            f"stdout:\n{completed.stdout}\n"
            f"stderr:\n{completed.stderr}"
        ) from exc
    if not isinstance(payload, dict):
        raise ValueError("premath command JSON root must be an object")
    return payload


def load_json(path: Path) -> Dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)
    except FileNotFoundError as exc:
        raise ValueError(f"missing file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid json: {path} ({exc})") from exc
    if not isinstance(data, dict):
        raise ValueError(f"json root must be object: {path}")
    return data


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label} must be a non-empty string")
    return value


def ensure_bool(value: Any, label: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{label} must be a boolean")
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


def parse_hex_bytes(value: Any, label: str) -> bytes:
    if not isinstance(value, str):
        raise ValueError(f"{label} must be a string")
    text = value
    if len(text) % 2 != 0:
        raise VectorError("kcir_v2.parse_error", f"{label} must have even-length hex")
    if not text:
        return b""
    try:
        return bytes.fromhex(text)
    except ValueError as exc:
        raise VectorError("kcir_v2.parse_error", f"{label} must be valid hex") from exc


def decode_varint(data: bytes, offset: int, label: str) -> Tuple[int, int]:
    value = 0
    shift = 0
    cursor = offset
    while True:
        if cursor >= len(data):
            raise VectorError("kcir_v2.parse_error", f"{label}: truncated varint")
        byte = data[cursor]
        cursor += 1
        value |= (byte & 0x7F) << shift
        if (byte & 0x80) == 0:
            return value, cursor
        shift += 7
        if shift > 63:
            raise VectorError("kcir_v2.parse_error", f"{label}: varint overflow")


def decode_enc_digest(data: bytes, offset: int, label: str) -> int:
    length, cursor = decode_varint(data, offset, f"{label}.len")
    end = cursor + length
    if end > len(data):
        raise VectorError("kcir_v2.parse_error", f"{label}: digest bytes exceed payload")
    return end


def decode_enc_list_digest(data: bytes, offset: int, label: str) -> int:
    count, cursor = decode_varint(data, offset, f"{label}.count")
    for idx in range(count):
        cursor = decode_enc_digest(data, cursor, f"{label}[{idx}]")
    return cursor


def parse_obj_nf(payload: bytes) -> None:
    if not payload:
        raise VectorError("kcir_v2.parse_error", "obj nf payload is empty")
    tag = payload[0]
    cursor = 1

    if tag == 0x01:
        pass
    elif tag == 0x02:
        cursor += 32
    elif tag == 0x03:
        cursor = decode_enc_list_digest(payload, cursor, "obj.tensor")
    elif tag in {0x04, 0x05}:
        cursor += 32
        if cursor > len(payload):
            raise VectorError("kcir_v2.parse_error", "obj spine: missing 32-byte id")
        cursor = decode_enc_digest(payload, cursor, "obj.spine.base")
    elif tag == 0x06:
        cursor += 32
        if cursor > len(payload):
            raise VectorError("kcir_v2.parse_error", "obj glue: missing 32-byte wSig")
        cursor = decode_enc_list_digest(payload, cursor, "obj.glue.locals")
    else:
        raise VectorError("kcir_v2.parse_error", f"obj nf unsupported tag: 0x{tag:02x}")

    if cursor != len(payload):
        raise VectorError("kcir_v2.parse_error", "obj nf has trailing bytes")


def parse_mor_nf(payload: bytes, adopt_pull_atom_mor: bool) -> None:
    if not payload:
        raise VectorError("kcir_v2.parse_error", "mor nf payload is empty")
    tag = payload[0]
    cursor = 1

    if tag == 0x11:
        cursor = decode_enc_digest(payload, cursor, "mor.id.src")
    elif tag == 0x13:
        cursor = decode_enc_digest(payload, cursor, "mor.comp.src")
        cursor = decode_enc_digest(payload, cursor, "mor.comp.tgt")
        cursor = decode_enc_list_digest(payload, cursor, "mor.comp.parts")
    elif tag == 0x17:
        cursor = decode_enc_digest(payload, cursor, "mor.push.src")
        cursor = decode_enc_digest(payload, cursor, "mor.push.tgt")
        cursor += 32
        if cursor > len(payload):
            raise VectorError("kcir_v2.parse_error", "mor push: missing 32-byte fId")
        cursor = decode_enc_digest(payload, cursor, "mor.push.inner")
    elif tag == 0x18:
        cursor = decode_enc_digest(payload, cursor, "mor.tensor.src")
        cursor = decode_enc_digest(payload, cursor, "mor.tensor.tgt")
        cursor = decode_enc_list_digest(payload, cursor, "mor.tensor.parts")
    elif tag == 0x19:
        cursor = decode_enc_digest(payload, cursor, "mor.glue.src")
        cursor = decode_enc_digest(payload, cursor, "mor.glue.tgt")
        cursor += 32
        if cursor > len(payload):
            raise VectorError("kcir_v2.parse_error", "mor glue: missing 32-byte wSig")
        cursor = decode_enc_list_digest(payload, cursor, "mor.glue.locals")
    elif tag == 0x16:
        if not adopt_pull_atom_mor:
            raise VectorError("kcir_v2.contract_violation", "pull atom tag requires adoptPullAtomMor claim")
        cursor = decode_enc_digest(payload, cursor, "mor.pull.src")
        cursor = decode_enc_digest(payload, cursor, "mor.pull.tgt")
        cursor += 32
        if cursor > len(payload):
            raise VectorError("kcir_v2.parse_error", "mor pull: missing 32-byte pId")
        cursor = decode_enc_digest(payload, cursor, "mor.pull.inner")
    else:
        raise VectorError("kcir_v2.parse_error", f"mor nf unsupported tag: 0x{tag:02x}")

    if cursor != len(payload):
        raise VectorError("kcir_v2.parse_error", "mor nf has trailing bytes")


def parse_wire_lenprefixed_ref_v1(payload: bytes) -> None:
    if len(payload) < 66:
        raise VectorError("kcir_v2.parse_error", "wire payload too short for fixed prefix")
    cursor = 66  # envSig(32) + uid(32) + sort(1) + opcode(1)

    out_len, cursor = decode_varint(payload, cursor, "wire.outLen")
    cursor += out_len
    if cursor > len(payload):
        raise VectorError("kcir_v2.parse_error", "wire outRef exceeds payload")

    args_len, cursor = decode_varint(payload, cursor, "wire.argsLen")
    cursor += args_len
    if cursor > len(payload):
        raise VectorError("kcir_v2.parse_error", "wire args exceed payload")

    dep_count, cursor = decode_varint(payload, cursor, "wire.depsCount")
    for idx in range(dep_count):
        dep_len, cursor = decode_varint(payload, cursor, f"wire.dep[{idx}].len")
        cursor += dep_len
        if cursor > len(payload):
            raise VectorError("kcir_v2.parse_error", "wire depRef exceeds payload")

    if cursor != len(payload):
        raise VectorError("kcir_v2.parse_error", "wire payload has trailing bytes")


def parse_wire_legacy_fixed32_v1(payload: bytes) -> None:
    if len(payload) < 98:
        raise VectorError("kcir_v2.parse_error", "legacy wire payload too short for fixed prefix")
    cursor = 98  # envSig(32) + uid(32) + sort(1) + opcode(1) + out(32)

    args_len, cursor = decode_varint(payload, cursor, "legacy.argsLen")
    cursor += args_len
    if cursor > len(payload):
        raise VectorError("kcir_v2.parse_error", "legacy args exceed payload")

    dep_count, cursor = decode_varint(payload, cursor, "legacy.depsCount")
    cursor += dep_count * 32
    if cursor > len(payload):
        raise VectorError("kcir_v2.parse_error", "legacy deps exceed payload")

    if cursor != len(payload):
        raise VectorError("kcir_v2.parse_error", "legacy wire payload has trailing bytes")


def evaluate_ref_projection_and_verify(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    profile_path_raw = artifacts.get("profilePath")
    projection_input = artifacts.get("projectionInput")
    provided_ref = artifacts.get("providedRef")
    if not isinstance(profile_path_raw, str) or not profile_path_raw.strip():
        profile_path = DEFAULT_REF_PROFILE_PATH
    else:
        profile_path = Path(profile_path_raw.strip())
        if not profile_path.is_absolute():
            profile_path = ROOT / profile_path
    if not profile_path.exists() or not profile_path.is_file():
        raise ValueError(f"artifacts.profilePath file not found: {profile_path}")
    if not isinstance(projection_input, dict):
        raise ValueError("artifacts.projectionInput must be an object")
    if not isinstance(provided_ref, dict):
        raise ValueError("artifacts.providedRef must be an object")

    domain = ensure_string(projection_input.get("domain"), "artifacts.projectionInput.domain")
    payload_hex = ensure_string(projection_input.get("payloadHex"), "artifacts.projectionInput.payloadHex")
    parse_hex_bytes(payload_hex, "artifacts.projectionInput.payloadHex")

    ref_scheme = ensure_string(provided_ref.get("schemeId"), "artifacts.providedRef.schemeId")
    ref_params = ensure_string(provided_ref.get("paramsHash"), "artifacts.providedRef.paramsHash")
    ref_domain = ensure_string(provided_ref.get("domain"), "artifacts.providedRef.domain")
    ref_digest = ensure_string(provided_ref.get("digest"), "artifacts.providedRef.digest")

    evidence_value = artifacts.get("evidenceHex", "")
    if not isinstance(evidence_value, str):
        raise ValueError("artifacts.evidenceHex must be a string")
    evidence_hex = evidence_value
    parse_hex_bytes(evidence_hex, "artifacts.evidenceHex")

    project_payload = run_premath_json(
        [
            "ref",
            "project",
            "--profile",
            str(profile_path),
            "--domain",
            domain,
            "--payload-hex",
            payload_hex,
            "--json",
        ]
    )
    projected_ref = project_payload.get("ref")
    if not isinstance(projected_ref, dict):
        raise ValueError("premath ref project output missing ref object")

    verify_payload = run_premath_json(
        [
            "ref",
            "verify",
            "--profile",
            str(profile_path),
            "--domain",
            domain,
            "--payload-hex",
            payload_hex,
            "--evidence-hex",
            evidence_hex,
            "--ref-scheme-id",
            ref_scheme,
            "--ref-params-hash",
            ref_params,
            "--ref-domain",
            ref_domain,
            "--ref-digest",
            ref_digest,
            "--json",
        ]
    )
    result = ensure_string(verify_payload.get("result"), "premath.ref.verify.result")
    failure_classes = ensure_string_list(
        verify_payload.get("failureClasses", []),
        "premath.ref.verify.failureClasses",
    )
    verify_projected = verify_payload.get("projectedRef")
    if verify_projected is not None and not isinstance(verify_projected, dict):
        raise ValueError("premath ref verify projectedRef must be an object when present")
    if isinstance(verify_projected, dict) and verify_projected != projected_ref:
        return VectorOutcome("rejected", ["kcir_v2.digest_mismatch"])
    if result == "accepted":
        return VectorOutcome("accepted", [])
    if result == "rejected":
        return VectorOutcome("rejected", sorted(set(failure_classes)))
    raise ValueError(f"premath ref verify result must be accepted/rejected (actual={result!r})")


def evaluate_domain_table(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    profile = artifacts.get("profile")
    if not isinstance(profile, dict):
        raise ValueError("artifacts.profile must be an object")
    supported = set(ensure_string_list(profile.get("supportedDomains", []), "artifacts.profile.supportedDomains"))
    if REQUIRED_DOMAINS <= supported:
        return VectorOutcome("accepted", [])
    return VectorOutcome("rejected", ["kcir_v2.domain_mismatch"])


def evaluate_nf_parse(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    adopt_pull = ensure_bool(artifacts.get("adoptPullAtomMor"), "artifacts.adoptPullAtomMor")
    obj_hex = artifacts.get("objHex")
    mor_hex = artifacts.get("morHex")
    if obj_hex is not None:
        obj_bytes = parse_hex_bytes(obj_hex, "artifacts.objHex")
        parse_obj_nf(obj_bytes)
    if mor_hex is not None:
        mor_bytes = parse_hex_bytes(mor_hex, "artifacts.morHex")
        parse_mor_nf(mor_bytes, adopt_pull)
    return VectorOutcome("accepted", [])


def evaluate_wire_parse(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    wire_format = ensure_string(artifacts.get("wireFormatId"), "artifacts.wireFormatId")
    payload = parse_hex_bytes(artifacts.get("nodeHex"), "artifacts.nodeHex")

    if wire_format not in SUPPORTED_WIRE_FORMAT_IDS:
        return VectorOutcome("rejected", ["kcir_v2.parse_error"])
    if wire_format == "kcir.wire.lenprefixed-ref.v1":
        parse_wire_lenprefixed_ref_v1(payload)
    else:
        parse_wire_legacy_fixed32_v1(payload)
    return VectorOutcome("accepted", [])


def evaluate_error_code_registry(case: Dict[str, Any]) -> VectorOutcome:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")
    code = ensure_string(artifacts.get("code"), "artifacts.code")
    if code in KNOWN_ERROR_CODES:
        return VectorOutcome("accepted", [])
    return VectorOutcome("rejected", ["kcir_v2.contract_violation"])


def evaluate_vector(vector_id: str, case: Dict[str, Any]) -> VectorOutcome:
    try:
        if vector_id in {
            "golden/ref_projection_and_verify_accept",
            "adversarial/ref_digest_mismatch_reject",
        }:
            return evaluate_ref_projection_and_verify(case)
        if vector_id in {
            "golden/kcir_domain_table_minimum_accept",
            "adversarial/kcir_domain_missing_reject",
        }:
            return evaluate_domain_table(case)
        if vector_id in {
            "golden/nf_obj_mor_parse_accept",
            "adversarial/nf_pull_atom_requires_claim_reject",
        }:
            return evaluate_nf_parse(case)
        if vector_id in {
            "golden/wire_lenprefixed_ref_v1_parse_accept",
            "golden/wire_legacy_fixed32_v1_parse_accept",
            "adversarial/wire_lenprefixed_truncated_reject",
        }:
            return evaluate_wire_parse(case)
        if vector_id in {
            "golden/error_code_registry_known_accept",
            "adversarial/error_code_registry_unknown_reject",
        }:
            return evaluate_error_code_registry(case)
    except VectorError as exc:
        return VectorOutcome("rejected", [exc.code])
    raise ValueError(f"unsupported interop-core vector id: {vector_id}")


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "interop-core":
        raise ValueError("manifest.suiteId must be 'interop-core'")

    vectors_raw = manifest.get("vectors")
    if not isinstance(vectors_raw, list) or not vectors_raw:
        raise ValueError("manifest.vectors must be a non-empty list")

    vectors: List[str] = []
    for idx, item in enumerate(vectors_raw):
        if not isinstance(item, str) or not item:
            raise ValueError(f"manifest.vectors[{idx}] must be a non-empty string")
        vectors.append(item)

    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def verify_case_and_expect(
    fixtures: Path,
    vector_id: str,
) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    case_path = fixtures / vector_id / "case.json"
    expect_path = fixtures / vector_id / "expect.json"
    case = load_json(case_path)
    expect = load_json(expect_path)

    if case.get("schema") != 1:
        raise ValueError(f"{case_path}: schema must be 1")
    if expect.get("schema") != 1:
        raise ValueError(f"{expect_path}: schema must be 1")
    if case.get("suiteId") != "interop-core":
        raise ValueError(f"{case_path}: suiteId must be 'interop-core'")
    if case.get("vectorId") != vector_id:
        raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")

    return case, expect


def run(fixtures: Path) -> int:
    vectors = validate_manifest(fixtures)
    errors: List[str] = []
    executed = 0

    for vector_id in vectors:
        try:
            case, expect = verify_case_and_expect(fixtures, vector_id)
            outcome = evaluate_vector(vector_id, case)
            expected_result = ensure_string(expect.get("expectedResult"), "expect.expectedResult")
            expected_failures = sorted(
                set(ensure_string_list(expect.get("expectedFailureClasses", []), "expect.expectedFailureClasses"))
            )
            actual_failures = sorted(set(outcome.failure_classes))

            if outcome.result != expected_result:
                raise ValueError(
                    f"expectedResult mismatch for {vector_id}: expected={expected_result} actual={outcome.result}"
                )
            if actual_failures != expected_failures:
                raise ValueError(
                    f"expectedFailureClasses mismatch for {vector_id}: "
                    f"expected={expected_failures} actual={actual_failures}"
                )
            print(f"[ok] interop-core/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[interop-core-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[interop-core-run] OK (vectors={executed})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Interop Core conformance vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Interop Core fixture root (default: {DEFAULT_FIXTURES})",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    if not fixtures.exists():
        print(f"[error] fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[error] fixtures path is not a directory: {fixtures}")
        return 2
    try:
        return run(fixtures)
    except Exception as exc:  # noqa: BLE001
        print(f"[interop-core-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
