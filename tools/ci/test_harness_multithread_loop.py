#!/usr/bin/env python3
"""Deterministic lease stop/handoff tests for harness multithread loop."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import sys

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "harness"))
import multithread_loop  # noqa: E402


def _issue_row(
    *,
    issue_id: str,
    status: str,
    assignee: str = "",
    lease: dict | None = None,
) -> dict:
    row: dict = {
        "id": issue_id,
        "title": f"Issue {issue_id}",
        "status": status,
        "assignee": assignee,
    }
    if lease is not None:
        row["lease"] = lease
    return row


def _lease_row(*, lease_id: str, owner: str, expires_at: str) -> dict:
    return {
        "lease_id": lease_id,
        "owner": owner,
        "acquired_at": "2026-02-23T00:00:00Z",
        "expires_at": expires_at,
    }


class HarnessMultithreadLoopTests(unittest.TestCase):
    def setUp(self) -> None:
        multithread_loop.load_mcp_only_host_actions.cache_clear()

    def test_build_site_lineage_refs_are_deterministic(self) -> None:
        refs_a = multithread_loop.build_site_lineage_refs(
            repo_root=Path("/tmp/repo"),
            issues_path=Path("/tmp/repo/.premath/issues.jsonl"),
            worktree=Path("/tmp/repo-w1"),
            worker_id="worker.1",
            issue_id="bd-7",
            mutation_mode="human-override",
            active_epoch="2026-02",
            support_until="2026-12",
        )
        refs_b = multithread_loop.build_site_lineage_refs(
            repo_root=Path("/tmp/repo"),
            issues_path=Path("/tmp/repo/.premath/issues.jsonl"),
            worktree=Path("/tmp/repo-w1"),
            worker_id="worker.1",
            issue_id="bd-7",
            mutation_mode="human-override",
            active_epoch="2026-02",
            support_until="2026-12",
        )
        self.assertEqual(refs_a, refs_b)
        self.assertEqual(len(refs_a), 3)
        self.assertTrue(any(ref.startswith("ctx://issue/bd-7/") for ref in refs_a))
        self.assertTrue(any(ref.startswith("cover://worker-loop/bd-7/") for ref in refs_a))
        self.assertTrue(
            any(ref.startswith("refinement://worker-loop/bd-7/worker.1/") for ref in refs_a)
        )

    def test_read_issue_lease_snapshot_active(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-loop-active-") as tmp:
            issues_path = Path(tmp) / "issues.jsonl"
            issues_path.write_text(
                json.dumps(
                    _issue_row(
                        issue_id="bd-1",
                        status="in_progress",
                        assignee="alice",
                        lease=_lease_row(
                            lease_id="lease1_bd-1_alice",
                            owner="alice",
                            expires_at="2099-01-01T00:00:00Z",
                        ),
                    )
                )
                + "\n",
                encoding="utf-8",
            )
            snapshot = multithread_loop.read_issue_lease_snapshot(issues_path, "bd-1")
            self.assertEqual(snapshot.lease_state, "active")
            self.assertEqual(snapshot.lease_id, "lease1_bd-1_alice")
            self.assertEqual(snapshot.lease_owner, "alice")

    def test_read_issue_lease_snapshot_stale(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-loop-stale-") as tmp:
            issues_path = Path(tmp) / "issues.jsonl"
            issues_path.write_text(
                json.dumps(
                    _issue_row(
                        issue_id="bd-2",
                        status="in_progress",
                        assignee="alice",
                        lease=_lease_row(
                            lease_id="lease1_bd-2_alice",
                            owner="alice",
                            expires_at="2000-01-01T00:00:00Z",
                        ),
                    )
                )
                + "\n",
                encoding="utf-8",
            )
            snapshot = multithread_loop.read_issue_lease_snapshot(issues_path, "bd-2")
            self.assertEqual(snapshot.lease_state, "stale")

    def test_read_issue_lease_snapshot_contended(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-loop-contended-") as tmp:
            issues_path = Path(tmp) / "issues.jsonl"
            issues_path.write_text(
                json.dumps(
                    _issue_row(
                        issue_id="bd-3",
                        status="in_progress",
                        assignee="alice",
                        lease=_lease_row(
                            lease_id="lease1_bd-3_bob",
                            owner="bob",
                            expires_at="2099-01-01T00:00:00Z",
                        ),
                    )
                )
                + "\n",
                encoding="utf-8",
            )
            snapshot = multithread_loop.read_issue_lease_snapshot(issues_path, "bd-3")
            self.assertEqual(snapshot.lease_state, "contended")

    def test_classify_failed_stop_handoff_active_requires_renew(self) -> None:
        snapshot = multithread_loop.IssueLeaseSnapshot(
            issue_id="bd-4",
            status="in_progress",
            assignee="alice",
            lease_id="lease1_bd-4_alice",
            lease_owner="alice",
            lease_expires_at="2099-01-01T00:00:00Z",
            lease_state="active",
        )
        handoff = multithread_loop.classify_failed_stop_handoff(
            snapshot=snapshot,
            worker_id="alice",
            claimed_lease_id="lease1_bd-4_alice",
        )
        self.assertEqual(handoff.lease_action, "issue_lease_renew")
        self.assertEqual(handoff.result_class, "retry_needed_lease_active")
        self.assertIn("issue_lease_renew", handoff.next_step)
        self.assertTrue(handoff.lease_ref.startswith("lease://handoff/bd-4/active/"))

    def test_classify_failed_stop_handoff_local_transport_fails_closed(self) -> None:
        snapshot = multithread_loop.IssueLeaseSnapshot(
            issue_id="bd-4",
            status="in_progress",
            assignee="alice",
            lease_id="lease1_bd-4_alice",
            lease_owner="alice",
            lease_expires_at="2099-01-01T00:00:00Z",
            lease_state="active",
        )
        handoff = multithread_loop.classify_failed_stop_handoff(
            snapshot=snapshot,
            worker_id="alice",
            claimed_lease_id="lease1_bd-4_alice",
            host_action_transport="local-repl",
        )
        self.assertEqual(
            handoff.result_class,
            "control_plane_host_action_mcp_transport_required",
        )
        self.assertEqual(handoff.lease_action, "issue_lease_renew")
        self.assertIn("switch to MCP transport", handoff.next_step)

    def test_classify_failed_stop_handoff_closed_restart_path(self) -> None:
        snapshot = multithread_loop.IssueLeaseSnapshot(
            issue_id="bd-5",
            status="closed",
            assignee="alice",
            lease_id=None,
            lease_owner=None,
            lease_expires_at=None,
            lease_state="released",
        )
        handoff = multithread_loop.classify_failed_stop_handoff(
            snapshot=snapshot,
            worker_id="alice",
            claimed_lease_id=None,
        )
        self.assertEqual(handoff.result_class, "failed_issue_closed")
        self.assertEqual(handoff.lease_action, "claim_next")
        self.assertIn("claim next ready issue", handoff.next_step)

    def test_assert_success_stop_handoff_requires_closed_released(self) -> None:
        released = multithread_loop.IssueLeaseSnapshot(
            issue_id="bd-6",
            status="closed",
            assignee="alice",
            lease_id=None,
            lease_owner=None,
            lease_expires_at=None,
            lease_state="released",
        )
        handoff = multithread_loop.assert_success_stop_handoff(
            snapshot=released,
            worker_id="alice",
            claimed_lease_id="lease1_bd-6_alice",
        )
        self.assertEqual(handoff.result_class, "completed")
        self.assertEqual(handoff.lease_action, "release_closed")

        unreleased = multithread_loop.IssueLeaseSnapshot(
            issue_id="bd-6",
            status="closed",
            assignee="alice",
            lease_id="lease1_bd-6_alice",
            lease_owner="alice",
            lease_expires_at="2099-01-01T00:00:00Z",
            lease_state="closed_with_lease",
        )
        with self.assertRaises(RuntimeError):
            multithread_loop.assert_success_stop_handoff(
                snapshot=unreleased,
                worker_id="alice",
                claimed_lease_id="lease1_bd-6_alice",
            )


if __name__ == "__main__":
    unittest.main()
