"""Transport adapter for Rust-native toy Gate checks."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

from core_cli_client import (  # type: ignore  # noqa: E402
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


class ToyGateCheckError(ValueError):
    """Toy Gate command failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("toy-gate-check payload must be an object")
    if payload.get("witnessSchema") != 1:
        raise ValueError("toy-gate-check payload witnessSchema must be 1")
    if payload.get("profile") != "toy":
        raise ValueError("toy-gate-check payload profile must be 'toy'")
    if payload.get("result") not in {"accepted", "rejected"}:
        raise ValueError("toy-gate-check payload result must be accepted or rejected")
    if not isinstance(payload.get("failures"), list):
        raise ValueError("toy-gate-check payload failures must be a list")
    return payload


def run_case(case: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            ROOT,
            subcommand="toy-gate-check",
            input_flag="--input",
            request_payload=case,
            validate_payload=_validate_payload,
            default_failure_class="toy_gate_check_invalid",
            default_failure_message="toy_gate_check_invalid: toy-gate-check failed",
            invalid_json_message="toy-gate-check returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise ToyGateCheckError(exc.failure_class, exc.reason) from exc
