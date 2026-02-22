#!/usr/bin/env python3
"""Validate GitHub branch/ruleset policy for deterministic no-bypass enforcement."""

from __future__ import annotations

import argparse
import json
import os
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Tuple

POLICY_KIND = "premath.github.branch_policy.v1"
DEFAULT_POLICY_PATH = (
    Path(__file__).resolve().parents[2] / "specs" / "process" / "GITHUB-BRANCH-POLICY.json"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check effective GitHub branch/ruleset policy against tracked governance contract.",
    )
    parser.add_argument(
        "--policy",
        type=Path,
        default=DEFAULT_POLICY_PATH,
        help=f"Policy artifact path (default: {DEFAULT_POLICY_PATH})",
    )
    parser.add_argument(
        "--rules-json",
        type=Path,
        default=None,
        help="Path to effective branch-rules JSON payload (fixture/offline mode).",
    )
    parser.add_argument(
        "--fetch-live",
        action="store_true",
        help=(
            "Fetch effective rules from GitHub API "
            "(`/repos/{repo}/rules/branches/{branch}`) instead of --rules-json."
        ),
    )
    parser.add_argument(
        "--repo",
        type=str,
        default=None,
        help="Repository slug owner/name (default: policy.repository).",
    )
    parser.add_argument(
        "--branch",
        type=str,
        default=None,
        help="Branch name (default: policy.branch).",
    )
    parser.add_argument(
        "--github-api-url",
        type=str,
        default=os.environ.get("GITHUB_API_URL", "https://api.github.com"),
        help="GitHub API base URL (default: env GITHUB_API_URL or https://api.github.com).",
    )
    parser.add_argument(
        "--token-env",
        type=str,
        default="GITHUB_TOKEN",
        help="Environment variable containing API token for --fetch-live (default: GITHUB_TOKEN).",
    )
    return parser.parse_args()


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _require_non_empty_string(
    payload: Dict[str, Any],
    key: str,
    path: Path,
) -> str:
    value = payload.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{path}: {key} must be a non-empty string")
    return value.strip()


def _require_string_list(
    payload: Dict[str, Any],
    key: str,
    path: Path,
) -> List[str]:
    value = payload.get(key)
    if not isinstance(value, list) or not value:
        raise ValueError(f"{path}: {key} must be a non-empty list of strings")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{path}: {key}[{idx}] must be a non-empty string")
        out.append(item.strip())
    if len(set(out)) != len(out):
        raise ValueError(f"{path}: {key} must not contain duplicates")
    return out


def parse_policy(path: Path) -> Dict[str, Any]:
    payload = load_json(path)
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: policy root must be an object")
    if payload.get("schema") != 1:
        raise ValueError(f"{path}: schema must be 1")
    if payload.get("policyKind") != POLICY_KIND:
        raise ValueError(f"{path}: policyKind must be {POLICY_KIND!r}")

    repository = _require_non_empty_string(payload, "repository", path)
    if "/" not in repository:
        raise ValueError(f"{path}: repository must be owner/name")

    policy = {
        "schema": 1,
        "policyKind": POLICY_KIND,
        "policyId": _require_non_empty_string(payload, "policyId", path),
        "repository": repository,
        "branch": _require_non_empty_string(payload, "branch", path),
        "requiredRuleTypes": _require_string_list(payload, "requiredRuleTypes", path),
        "requiredStatusChecks": _require_string_list(payload, "requiredStatusChecks", path),
        "strictStatusChecks": bool(payload.get("strictStatusChecks", False)),
        "requirePullRequest": bool(payload.get("requirePullRequest", False)),
        "forbidBypassActors": bool(payload.get("forbidBypassActors", False)),
    }
    return policy


def fetch_live_rules(
    *,
    api_url: str,
    repo: str,
    branch: str,
    token: str,
) -> Any:
    def _fetch_json(url: str) -> Any:
        request = urllib.request.Request(
            url,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {token}",
                "X-GitHub-Api-Version": "2022-11-28",
                "User-Agent": "premath-branch-policy-check",
            },
        )
        try:
            with urllib.request.urlopen(request) as response:
                data = response.read().decode("utf-8")
                return json.loads(data)
        except urllib.error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            raise ValueError(f"live rules fetch failed ({exc.code}) for {url}: {body}") from exc
        except urllib.error.URLError as exc:
            raise ValueError(f"live rules fetch failed for {url}: {exc}") from exc

    base = api_url.rstrip("/")
    rules_url = f"{base}/repos/{repo}/rules/branches/{branch}"
    payload = _fetch_json(rules_url)
    # When no rulesets are configured, GitHub may return [] here even with classic branch protection.
    if isinstance(payload, list) and not payload:
        protection_url = f"{base}/repos/{repo}/branches/{branch}/protection"
        return _fetch_json(protection_url)
    return payload


def _admin_bypass_enabled(payload: Any) -> bool | None:
    if not isinstance(payload, dict):
        return None
    enforce_admins = payload.get("enforce_admins")
    if isinstance(enforce_admins, bool):
        return not enforce_admins
    if isinstance(enforce_admins, dict):
        enabled = enforce_admins.get("enabled")
        if isinstance(enabled, bool):
            return not enabled
    return None


def _normalize_actor(row: Any) -> str:
    if isinstance(row, dict):
        actor_type = row.get("actor_type") or row.get("actorType")
        actor_id = row.get("actor_id") or row.get("actorId") or row.get("id")
        actor_name = row.get("login") or row.get("slug") or row.get("name")
        if actor_type and actor_id is not None:
            return f"{actor_type}:{actor_id}"
        if actor_type and actor_name:
            return f"{actor_type}:{actor_name}"
        if actor_name:
            return str(actor_name)
        return json.dumps(row, sort_keys=True, separators=(",", ":"))
    if isinstance(row, str):
        return row
    return json.dumps(row, sort_keys=True, separators=(",", ":"))


def _collect_bypass_pull_request_allowances(allowances: Dict[str, Any]) -> List[str]:
    out: List[str] = []
    for key in ("users", "teams", "apps"):
        rows = allowances.get(key)
        if not isinstance(rows, list):
            continue
        for row in rows:
            out.append(f"{key}:{_normalize_actor(row)}")
    return out


def collect_bypass_actors(payload: Any) -> List[str]:
    out: List[str] = []

    def visit(node: Any) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "bypass_actors" and isinstance(value, list):
                    for actor in value:
                        out.append(_normalize_actor(actor))
                if key == "bypass_pull_request_allowances" and isinstance(value, dict):
                    out.extend(_collect_bypass_pull_request_allowances(value))
                visit(value)
            return
        if isinstance(node, list):
            for item in node:
                visit(item)

    visit(payload)
    return sorted(set(out))


def _as_rule_list(payload: Any) -> List[Dict[str, Any]]:
    if isinstance(payload, list):
        out: List[Dict[str, Any]] = []
        for row in payload:
            if isinstance(row, dict):
                out.append(row)
        return out

    if isinstance(payload, dict):
        rules = payload.get("rules")
        if isinstance(rules, list):
            out: List[Dict[str, Any]] = []
            for row in rules:
                if isinstance(row, dict):
                    out.append(row)
            return out

        # Fallback parser for classic branch protection payloads.
        synthetic: List[Dict[str, Any]] = []
        required_status = payload.get("required_status_checks")
        if isinstance(required_status, dict):
            contexts = required_status.get("contexts", [])
            checks: List[Dict[str, Any]] = []
            if isinstance(contexts, list):
                for row in contexts:
                    if isinstance(row, str) and row.strip():
                        checks.append({"context": row.strip()})
            synthetic.append(
                {
                    "type": "required_status_checks",
                    "parameters": {
                        "strict_required_status_checks_policy": bool(required_status.get("strict")),
                        "required_status_checks": checks,
                    },
                }
            )
        if isinstance(payload.get("required_pull_request_reviews"), dict):
            synthetic.append(
                {
                    "type": "pull_request",
                    "parameters": payload.get("required_pull_request_reviews"),
                }
            )
        allow_force = payload.get("allow_force_pushes")
        if isinstance(allow_force, dict) and allow_force.get("enabled") is False:
            synthetic.append({"type": "non_fast_forward"})
        allow_delete = payload.get("allow_deletions")
        if isinstance(allow_delete, dict) and allow_delete.get("enabled") is False:
            synthetic.append({"type": "deletion"})
        return synthetic

    return []


def extract_rule_types(rules: Iterable[Dict[str, Any]]) -> List[str]:
    out = set()
    for row in rules:
        rule_type = row.get("type")
        if isinstance(rule_type, str) and rule_type:
            out.add(rule_type)
    return sorted(out)


def extract_required_status_contexts(rules: Iterable[Dict[str, Any]]) -> List[str]:
    out = set()
    for row in rules:
        if row.get("type") != "required_status_checks":
            continue
        params = row.get("parameters")
        if not isinstance(params, dict):
            continue
        checks = params.get("required_status_checks")
        if not isinstance(checks, list):
            continue
        for check in checks:
            if isinstance(check, dict):
                context = check.get("context")
                if isinstance(context, str) and context.strip():
                    out.add(context.strip())
            elif isinstance(check, str) and check.strip():
                out.add(check.strip())
    return sorted(out)


def extract_strict_status_checks(rules: Iterable[Dict[str, Any]]) -> bool | None:
    strict_values: List[bool] = []
    for row in rules:
        if row.get("type") != "required_status_checks":
            continue
        params = row.get("parameters")
        if not isinstance(params, dict):
            continue
        value = params.get("strict_required_status_checks_policy")
        if isinstance(value, bool):
            strict_values.append(value)
    if not strict_values:
        return None
    return all(strict_values)


def evaluate_policy(policy: Dict[str, Any], payload: Any) -> Tuple[List[str], Dict[str, Any]]:
    rules = _as_rule_list(payload)
    rule_types = extract_rule_types(rules)
    required_checks = extract_required_status_contexts(rules)
    strict_checks = extract_strict_status_checks(rules)
    bypass_actors = collect_bypass_actors(payload)
    admin_bypass = _admin_bypass_enabled(payload)

    errors: List[str] = []

    if not rules:
        errors.append("missing rules surface in payload")

    for rule_type in policy["requiredRuleTypes"]:
        if rule_type not in rule_types:
            errors.append(f"missing required rule type: {rule_type}")

    if policy["requirePullRequest"] and "pull_request" not in rule_types:
        errors.append("pull_request rule missing while requirePullRequest=true")

    for check in policy["requiredStatusChecks"]:
        if check not in required_checks:
            errors.append(f"missing required status check context: {check}")

    if policy["strictStatusChecks"] and strict_checks is not True:
        errors.append("strict status checks policy is not enabled")

    if policy["forbidBypassActors"] and bypass_actors:
        errors.append(f"bypass actors present: {', '.join(bypass_actors)}")
    if policy["forbidBypassActors"] and admin_bypass is True:
        errors.append("admin bypass path enabled: enforce_admins=false")

    details = {
        "ruleTypes": rule_types,
        "requiredStatusChecks": required_checks,
        "strictStatusChecks": strict_checks,
        "bypassActors": bypass_actors,
        "adminBypassEnabled": admin_bypass,
    }
    return errors, details


def main() -> int:
    args = parse_args()
    policy_path = args.policy.resolve()
    try:
        policy = parse_policy(policy_path)
    except ValueError as exc:
        print(f"[branch-policy-check] FAIL policy-invalid: {exc}")
        return 1

    if args.rules_json and args.fetch_live:
        print("[branch-policy-check] FAIL invalid-args: choose exactly one of --rules-json or --fetch-live")
        return 1
    if not args.rules_json and not args.fetch_live:
        print("[branch-policy-check] FAIL invalid-args: one of --rules-json or --fetch-live is required")
        return 1

    repo = args.repo or policy["repository"]
    branch = args.branch or policy["branch"]

    if args.rules_json:
        payload_path = args.rules_json.resolve()
        try:
            payload = load_json(payload_path)
        except (OSError, json.JSONDecodeError) as exc:
            print(f"[branch-policy-check] FAIL input-invalid: {payload_path}: {exc}")
            return 1
        source = str(payload_path)
    else:
        token = os.environ.get(args.token_env, "").strip()
        if not token:
            print(f"[branch-policy-check] FAIL missing-token: env {args.token_env} is required for --fetch-live")
            return 1
        try:
            payload = fetch_live_rules(
                api_url=args.github_api_url,
                repo=repo,
                branch=branch,
                token=token,
            )
        except ValueError as exc:
            print(f"[branch-policy-check] FAIL live-fetch: {exc}")
            return 1
        source = f"live:{repo}:{branch}"

    errors, details = evaluate_policy(policy, payload)
    if errors:
        print(
            "[branch-policy-check] FAIL "
            f"(policyId={policy['policyId']}, source={source}, errors={len(errors)})"
        )
        for error in errors:
            print(f"  - {error}")
        print(
            "[branch-policy-check] DETAILS "
            + json.dumps(details, sort_keys=True, separators=(",", ":"))
        )
        return 1

    print(
        "[branch-policy-check] OK "
        f"(policyId={policy['policyId']}, source={source}, "
        f"ruleTypes={len(details['ruleTypes'])}, requiredChecks={len(details['requiredStatusChecks'])})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
