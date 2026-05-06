"""Transport adapter for Rust-native toy Gate checks."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict

ROOT = Path(__file__).resolve().parents[2]


class ToyGateCheckError(ValueError):
    """Toy Gate command failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


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
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False, encoding="utf-8") as f:
        json.dump(case, f, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        f.write("\n")
        input_path = Path(f.name)

    try:
        completed = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "--package",
                "premath-cli",
                "--",
                "toy-gate-check",
                "--input",
                str(input_path),
                "--json",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            message = completed.stderr.strip() or completed.stdout.strip() or "toy-gate-check failed"
            if ":" in message:
                failure_class, reason = message.split(":", 1)
                raise ToyGateCheckError(failure_class.strip(), reason.strip() or message)
            raise ToyGateCheckError("toy_gate_check_invalid", message)
        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise ToyGateCheckError("toy_gate_check_invalid", "toy-gate-check returned invalid JSON") from exc
        try:
            return _validate_payload(payload)
        except ValueError as exc:
            raise ToyGateCheckError("toy_gate_check_invalid", str(exc)) from exc
    finally:
        try:
            input_path.unlink()
        except FileNotFoundError:
            pass
