#!/usr/bin/env python3
"""Denylist guard for wrapper-defined failure class families."""

from __future__ import annotations

from typing import Sequence

SEMANTIC_FAILURE_CLASS_PREFIXES: tuple[str, ...] = (
    "world_route_",
    "site_resolve_",
    "runtime_route_",
    "runtime_",
)


def is_semantic_failure_class(failure_class: str) -> bool:
    candidate = failure_class.strip()
    return any(
        candidate.startswith(prefix) for prefix in SEMANTIC_FAILURE_CLASS_PREFIXES
    )


def assert_nonsemantic_wrapper_failure_classes(
    *,
    wrapper_id: str,
    failure_classes: Sequence[str],
) -> None:
    for idx, failure_class in enumerate(failure_classes):
        if not isinstance(failure_class, str) or not failure_class.strip():
            raise ValueError(
                f"{wrapper_id}: wrapper failureClasses[{idx}] must be a non-empty string"
            )
        if is_semantic_failure_class(failure_class):
            raise ValueError(
                f"{wrapper_id}: wrapper non-semantic guard rejected failure class "
                f"{failure_class!r}"
            )
