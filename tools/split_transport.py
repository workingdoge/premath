#!/usr/bin/env python3
"""
Split crates/premath-transport/src/lib.rs into modules.

Target layout:
  lib.rs          re-exports, constants, imports, NIF block, tests
  types.rs        shared types, specs, digests, enums
  lease.rs        lease operations (claim, renew, release, helpers)
  dispatch.rs     transport dispatch + instruction logic (coupled)
  fiber.rs        fiber lifecycle (spawn, join, cancel)
  registry.rs     action registry validation + kernel binding + transport_check

Usage:
  python3 tools/split_transport.py          # dry-run (prints plan)
  python3 tools/split_transport.py --write  # actually write files

The script is intentionally explicit about line ranges so every cut is auditable.
"""

import argparse
import os
import re
import sys
from pathlib import Path

CRATE_ROOT = Path(__file__).resolve().parent.parent / "crates" / "premath-transport" / "src"
LIB_RS = CRATE_ROOT / "lib.rs"


def read_lines(path: Path) -> list[str]:
    with open(path, "r") as f:
        return f.readlines()


# ---------------------------------------------------------------------------
# Line-range helpers (1-indexed, inclusive on both ends to match editor lines)
# ---------------------------------------------------------------------------

def extract(lines: list[str], start: int, end: int) -> list[str]:
    """Extract lines[start-1:end] (1-indexed, inclusive)."""
    return lines[start - 1 : end]


def lines_text(lines: list[str], start: int, end: int) -> str:
    return "".join(extract(lines, start, end))


# ---------------------------------------------------------------------------
# Locate structural boundaries by scanning content
# ---------------------------------------------------------------------------

def find_line(lines: list[str], needle: str, start_from: int = 1) -> int:
    """Return 1-indexed line number of first line containing `needle` at or after start_from."""
    for i in range(start_from - 1, len(lines)):
        if needle in lines[i]:
            return i + 1
    raise ValueError(f"Could not find {needle!r} starting from line {start_from}")


def find_line_exact(lines: list[str], needle: str, start_from: int = 1) -> int:
    """Return 1-indexed line number where line.strip() == needle."""
    for i in range(start_from - 1, len(lines)):
        if lines[i].strip() == needle:
            return i + 1
    raise ValueError(f"Could not find exact {needle!r} starting from line {start_from}")


def find_block_end(lines: list[str], start: int) -> int:
    """Given the 1-indexed start of a block with '{', find its matching '}'.
    Returns the 1-indexed line of the closing brace."""
    depth = 0
    found_open = False
    for i in range(start - 1, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if "{" in lines[i]:
            found_open = True
        if found_open and depth <= 0:
            return i + 1
    raise ValueError(f"Unclosed block starting at line {start}")


def find_fn_or_impl_end(lines: list[str], start: int) -> int:
    """Find end of a fn/impl/struct block starting at `start` (1-indexed)."""
    # Walk forward to find the opening brace
    for i in range(start - 1, len(lines)):
        if "{" in lines[i]:
            return find_block_end(lines, i + 1)
    raise ValueError(f"No opening brace found from line {start}")


def find_item_start(lines: list[str], line_num: int) -> int:
    """Walk backwards from line_num to find #[derive], #[serde], #[cfg], /// doc comments."""
    i = line_num - 1  # 0-indexed
    while i > 0:
        prev = lines[i - 1].strip()
        if (prev.startswith("#[") or prev.startswith("///") or
            prev.startswith("//!") or prev == ""):
            # include attributes and doc comments; skip blank lines between them
            if prev == "":
                # Only include blank line if the line before it is also an attribute/doc
                if i > 1:
                    prev2 = lines[i - 2].strip()
                    if prev2.startswith("#[") or prev2.startswith("///"):
                        i -= 1
                        continue
                break
            i -= 1
        else:
            break
    return i + 1  # back to 1-indexed


# ---------------------------------------------------------------------------
# The core splitter
# ---------------------------------------------------------------------------

def build_modules(lines: list[str]) -> dict[str, str]:
    """Return {filename: content} for all output files."""
    total = len(lines)

    # -----------------------------------------------------------------------
    # Step 1: Identify key landmarks
    # -----------------------------------------------------------------------

    # Imports: lines 1-24 (use ... ; block)
    imports_end = find_line(lines, "use std::time::Instant;")

    # Constants block: from DEFAULT_ISSUES_PATH to FAILURE_INSTRUCTION_RUNTIME_INVALID
    const_start = find_line(lines, 'const DEFAULT_ISSUES_PATH')
    const_end = find_line(lines, 'FAILURE_INSTRUCTION_RUNTIME_INVALID')
    # include the full line
    while not lines[const_end - 1].rstrip().endswith(";"):
        const_end += 1

    # LeaseActionKind enum
    lease_action_kind_start = find_item_start(lines, find_line(lines, "enum LeaseActionKind"))
    lease_action_kind_impl_end = find_fn_or_impl_end(lines, find_line(lines, "impl LeaseActionKind"))

    # TransportActionId enum + impl
    transport_action_id_start = find_item_start(lines, find_line(lines, "enum TransportActionId"))
    transport_action_id_impl_end = find_fn_or_impl_end(lines, find_line(lines, "impl TransportActionId"))

    # TransportActionSpec struct
    transport_action_spec_start = find_item_start(lines, find_line(lines, "struct TransportActionSpec"))
    transport_action_spec_end = find_block_end(lines, find_line(lines, "struct TransportActionSpec"))

    # REQUIRED_MORPHISMS constants
    req_morph_start = find_line(lines, "const REQUIRED_MORPHISMS_LEASE")
    req_morph_end = find_line(lines, 'const REQUIRED_MORPHISMS_INSTRUCTION')
    # find the end of the last one
    while not lines[req_morph_end - 1].rstrip().endswith(";"):
        req_morph_end += 1

    # TRANSPORT_ACTION_SPECS array
    specs_array_start = find_line(lines, "const TRANSPORT_ACTION_SPECS:")
    specs_array_end = find_line_exact(lines, "];", specs_array_start)

    # transport_action_spec fn
    tas_fn_start = find_line(lines, "fn transport_action_spec(")
    tas_fn_end = find_block_end(lines, tas_fn_start)

    # WorldRouteBinding struct
    wrb_struct_start = find_item_start(lines, find_line(lines, "pub struct WorldRouteBinding"))
    wrb_struct_end = find_block_end(lines, find_line(lines, "pub struct WorldRouteBinding"))

    # world_binding, world_binding_for_action, resolver_witness_ref, resolve_witness_for_action,
    # resolver_fields_for_action
    world_binding_fn_start = find_line(lines, "fn world_binding(kind:")
    resolver_fields_end = find_block_end(lines, find_line(lines, "fn resolver_fields_for_action("))

    # semantic_digest fn
    semantic_digest_start = find_line(lines, "fn semantic_digest(")
    semantic_digest_end = find_block_end(lines, semantic_digest_start)

    # transport_dispatch_digest fn
    tdd_fn_start = find_line(lines, "fn transport_dispatch_digest(")
    tdd_fn_end = find_block_end(lines, tdd_fn_start)

    # transport_action_row_digest fn
    tard_fn_start = find_line(lines, "fn transport_action_row_digest(")
    tard_fn_end = find_block_end(lines, tard_fn_start)

    # TransportActionRegistryRow struct
    tarr_start = find_item_start(lines, find_line(lines, "pub struct TransportActionRegistryRow"))
    tarr_end = find_block_end(lines, find_line(lines, "pub struct TransportActionRegistryRow"))

    # TransportCheckIssue struct
    tci_start = find_item_start(lines, find_line(lines, "pub struct TransportCheckIssue"))
    tci_end = find_block_end(lines, find_line(lines, "pub struct TransportCheckIssue"))

    # TransportCheckReport struct
    tcr_start = find_item_start(lines, find_line(lines, "pub struct TransportCheckReport"))
    tcr_end = find_block_end(lines, find_line(lines, "pub struct TransportCheckReport"))

    # transport_action_row fn
    tar_fn_start = find_line(lines, "fn transport_action_row(spec:")
    tar_fn_end = find_block_end(lines, tar_fn_start)

    # transport_action_registry_rows fn
    tarr_fn_start = find_line(lines, "pub fn transport_action_registry_rows(")
    tarr_fn_end = find_block_end(lines, tarr_fn_start)

    # transport_check_digest fn
    tcd_fn_start = find_line(lines, "fn transport_check_digest(")
    tcd_fn_end = find_block_end(lines, tcd_fn_start)

    # validate_transport_registry fn
    vtr_fn_start = find_line(lines, "fn validate_transport_registry(")
    vtr_fn_end = find_block_end(lines, vtr_fn_start)

    # TransportKernelBindingError struct
    tkbe_start = find_item_start(lines, find_line(lines, "struct TransportKernelBindingError"))
    tkbe_end = find_block_end(lines, find_line(lines, "struct TransportKernelBindingError"))

    # resolve_site_for_spec fn
    rsfs_fn_start = find_line(lines, "fn resolve_site_for_spec(")
    rsfs_fn_end = find_block_end(lines, rsfs_fn_start)

    # validate_transport_action_binding_with_kernel fn
    vtabwk_fn_start = find_line(lines, "fn validate_transport_action_binding_with_kernel(")
    vtabwk_fn_end = find_block_end(lines, vtabwk_fn_start)

    # transport_check fn
    tc_fn_start = find_line(lines, "pub fn transport_check()")
    tc_fn_end = find_block_end(lines, tc_fn_start)

    # LeaseInfo struct
    li_start = find_item_start(lines, find_line(lines, "pub struct LeaseInfo"))
    li_end = find_block_end(lines, find_line(lines, "pub struct LeaseInfo"))

    # IssueSummary struct
    is_start = find_item_start(lines, find_line(lines, "pub struct IssueSummary"))
    is_end = find_block_end(lines, find_line(lines, "pub struct IssueSummary"))

    # LeaseProjection struct
    lp_start = find_item_start(lines, find_line(lines, "pub struct LeaseProjection"))
    lp_end = find_block_end(lines, find_line(lines, "pub struct LeaseProjection"))

    # LeaseActionEnvelope struct
    lae_start = find_item_start(lines, find_line(lines, "pub struct LeaseActionEnvelope"))
    lae_end = find_block_end(lines, find_line(lines, "pub struct LeaseActionEnvelope"))

    # LeaseMutationError struct + impl
    lme_start = find_item_start(lines, find_line(lines, "struct LeaseMutationError"))
    lme_impl_end = find_fn_or_impl_end(lines, find_line(lines, "impl LeaseMutationError"))

    # accepted_envelope fn
    ae_fn_start = find_line(lines, "fn accepted_envelope(")
    ae_fn_end = find_block_end(lines, ae_fn_start)

    # accepted_envelope_optional fn
    aeo_fn_start = find_line(lines, "fn accepted_envelope_optional(")
    aeo_fn_end = find_block_end(lines, aeo_fn_start)

    # rejected_envelope fn
    re_fn_start = find_line(lines, "fn rejected_envelope(")
    re_fn_end = find_block_end(lines, re_fn_start)

    # map_atomic_store_error fn
    mase_fn_start = find_line(lines, "fn map_atomic_store_error(")
    mase_fn_end = find_block_end(lines, mase_fn_start)

    # map_claim_next_atomic_store_error fn
    mcnase_fn_start = find_line(lines, "fn map_claim_next_atomic_store_error(")
    mcnase_fn_end = find_block_end(lines, mcnase_fn_start)

    # map_claim_next_error fn
    mcne_fn_start = find_line(lines, "fn map_claim_next_error(")
    mcne_fn_end = find_block_end(lines, mcne_fn_start)

    # non_empty fn
    ne_fn_start = find_line(lines, "fn non_empty(")
    ne_fn_end = find_block_end(lines, ne_fn_start)

    # resolve_issues_path fn
    rip_fn_start = find_line(lines, "fn resolve_issues_path(")
    rip_fn_end = find_block_end(lines, rip_fn_start)

    # parse_lease_ttl_seconds fn
    plts_fn_start = find_line(lines, "fn parse_lease_ttl_seconds(")
    plts_fn_end = find_block_end(lines, plts_fn_start)

    # parse_lease_expiry fn
    ple_fn_start = find_line(lines, "fn parse_lease_expiry(")
    ple_fn_end = find_block_end(lines, ple_fn_start)

    # lease_token fn
    lt_fn_start = find_line(lines, "fn lease_token(")
    lt_fn_end = find_block_end(lines, lt_fn_start)

    # resolve_lease_id fn
    rli_fn_start = find_line(lines, "fn resolve_lease_id(")
    rli_fn_end = find_block_end(lines, rli_fn_start)

    # lease_state_label fn
    lsl_fn_start = find_line(lines, "fn lease_state_label(")
    lsl_fn_end = find_block_end(lines, lsl_fn_start)

    # issue_is_lease_contended fn
    iilc_fn_start = find_line(lines, "fn issue_is_lease_contended(")
    iilc_fn_end = find_block_end(lines, iilc_fn_start)

    # issue_summary fn
    isf_fn_start = find_line(lines, "fn issue_summary(")
    isf_fn_end = find_block_end(lines, isf_fn_start)

    # compute_lease_projection fn
    clp_fn_start = find_line(lines, "fn compute_lease_projection(")
    clp_fn_end = find_block_end(lines, clp_fn_start)

    # Request types
    icr_start = find_item_start(lines, find_line(lines, "pub struct IssueClaimRequest"))
    icr_end = find_block_end(lines, find_line(lines, "pub struct IssueClaimRequest"))

    icnr_start = find_item_start(lines, find_line(lines, "pub struct IssueClaimNextRequest"))
    icnr_end = find_block_end(lines, find_line(lines, "pub struct IssueClaimNextRequest"))

    ilrr_start = find_item_start(lines, find_line(lines, "pub struct IssueLeaseRenewRequest"))
    ilrr_end = find_block_end(lines, find_line(lines, "pub struct IssueLeaseRenewRequest"))

    ilrelr_start = find_item_start(lines, find_line(lines, "pub struct IssueLeaseReleaseRequest"))
    ilrelr_end = find_block_end(lines, find_line(lines, "pub struct IssueLeaseReleaseRequest"))

    tdr_start = find_item_start(lines, find_line(lines, "pub struct TransportDispatchRequest"))
    tdr_end = find_block_end(lines, find_line(lines, "pub struct TransportDispatchRequest"))

    twbr_start = find_item_start(lines, find_line(lines, "pub struct TransportWorldBindingRequest"))
    twbr_end = find_block_end(lines, find_line(lines, "pub struct TransportWorldBindingRequest"))

    irr_start = find_item_start(lines, find_line(lines, "pub struct InstructionRunRequest"))
    irr_end = find_block_end(lines, find_line(lines, "pub struct InstructionRunRequest"))

    fsr_start = find_item_start(lines, find_line(lines, "pub struct FiberSpawnRequest"))
    fsr_end = find_block_end(lines, find_line(lines, "pub struct FiberSpawnRequest"))

    fjr_start = find_item_start(lines, find_line(lines, "pub struct FiberJoinRequest"))
    fjr_end = find_block_end(lines, find_line(lines, "pub struct FiberJoinRequest"))

    fcr_start = find_item_start(lines, find_line(lines, "pub struct FiberCancelRequest"))
    fcr_end = find_block_end(lines, find_line(lines, "pub struct FiberCancelRequest"))

    # Fiber helpers
    fiber_token_start = find_line(lines, "fn fiber_token(")
    fiber_token_end = find_block_end(lines, fiber_token_start)

    derive_fiber_id_start = find_line(lines, "fn derive_fiber_id(")
    derive_fiber_id_end = find_block_end(lines, derive_fiber_id_start)

    fiber_witness_ref_start = find_line(lines, "fn fiber_witness_ref(")
    fiber_witness_ref_end = find_block_end(lines, fiber_witness_ref_start)

    fiber_rejected_start = find_line(lines, "fn fiber_rejected(")
    fiber_rejected_end = find_block_end(lines, fiber_rejected_start)

    # Instruction / dispatch helpers
    truncate_start = find_line(lines, "fn truncate_for_payload(")
    truncate_end = find_block_end(lines, truncate_start)

    resolve_repo_root_start = find_line(lines, "fn resolve_repo_root(")
    resolve_repo_root_end = find_block_end(lines, resolve_repo_root_start)

    resolve_instr_path_start = find_line(lines, "fn resolve_instruction_path(")
    resolve_instr_path_end = find_block_end(lines, resolve_instr_path_start)

    instr_ref_start = find_line(lines, "fn instruction_ref(")
    instr_ref_end = find_block_end(lines, instr_ref_start)

    fallback_instr_id_start = find_line(lines, "fn fallback_instruction_id_from_path(")
    fallback_instr_id_end = find_block_end(lines, fallback_instr_id_start)

    instr_id_validated_start = find_line(lines, "fn instruction_id_from_validated_path(")
    instr_id_validated_end = find_block_end(lines, instr_id_validated_start)

    sort_json_start = find_line(lines, "fn sort_json_value(")
    sort_json_end = find_block_end(lines, sort_json_start)

    norm_instr_digest_start = find_line(lines, "fn normalized_instruction_digest(")
    norm_instr_digest_end = find_block_end(lines, norm_instr_digest_start)

    instr_runtime_payload_start = find_line(lines, "fn instruction_runtime_payload(")
    instr_runtime_payload_end = find_block_end(lines, instr_runtime_payload_start)

    write_instr_witness_start = find_line(lines, "fn write_instruction_witness(")
    write_instr_witness_end = find_block_end(lines, write_instr_witness_start)

    run_gate_check_start = find_line(lines, "fn run_gate_check(")
    run_gate_check_end = find_block_end(lines, run_gate_check_start)

    decision_state_start = find_line(lines, "fn decision_state(")
    decision_state_end = find_block_end(lines, decision_state_start)

    decision_source_reason_start = find_line(lines, "fn decision_source_reason(")
    decision_source_reason_end = find_block_end(lines, decision_source_reason_start)

    instr_run_rejected_start = find_line(lines, "fn instruction_run_rejected(")
    instr_run_rejected_end = find_block_end(lines, instr_run_rejected_start)

    instr_run_response_start = find_line(lines, "fn instruction_run_response(")
    instr_run_response_end = find_block_end(lines, instr_run_response_start)

    # Fiber response functions
    fiber_spawn_response_start = find_line(lines, "fn fiber_spawn_response(")
    fiber_spawn_response_end = find_block_end(lines, fiber_spawn_response_start)

    fiber_join_response_start = find_line(lines, "fn fiber_join_response(")
    fiber_join_response_end = find_block_end(lines, fiber_join_response_start)

    fiber_cancel_response_start = find_line(lines, "fn fiber_cancel_response(")
    fiber_cancel_response_end = find_block_end(lines, fiber_cancel_response_start)

    # Lease operations
    issue_claim_next_start = find_line(lines, "pub fn issue_claim_next(")
    issue_claim_next_end = find_block_end(lines, issue_claim_next_start)

    issue_claim_start = find_line(lines, "pub fn issue_claim(request: IssueClaimRequest)")
    issue_claim_end = find_block_end(lines, issue_claim_start)

    issue_lease_renew_start = find_line(lines, "pub fn issue_lease_renew(request: IssueLeaseRenewRequest)")
    issue_lease_renew_end = find_block_end(lines, issue_lease_renew_start)

    issue_lease_release_start = find_line(lines, "pub fn issue_lease_release(")
    issue_lease_release_end = find_block_end(lines, issue_lease_release_start)

    # JSON wrappers for lease
    icj_start = find_line(lines, "pub fn issue_claim_json(")
    icj_end = find_block_end(lines, icj_start)

    icnj_start = find_line(lines, "pub fn issue_claim_next_json(")
    icnj_end = find_block_end(lines, icnj_start)

    ilrj_start = find_line(lines, "pub fn issue_lease_renew_json(")
    ilrj_end = find_block_end(lines, ilrj_start)

    ilrelj_start = find_line(lines, "pub fn issue_lease_release_json(")
    ilrelj_end = find_block_end(lines, ilrelj_start)

    # transport_rejected fn
    tr_fn_start = find_line(lines, "fn transport_rejected(")
    tr_fn_end = find_block_end(lines, tr_fn_start)

    # annotate_transport_dispatch_fields fn
    atdf_fn_start = find_line(lines, "fn annotate_transport_dispatch_fields(")
    atdf_fn_end = find_block_end(lines, atdf_fn_start)

    # world_route_binding_json fn
    wrbj_fn_start = find_line(lines, "pub fn world_route_binding_json(")
    wrbj_fn_end = find_block_end(lines, wrbj_fn_start)

    # transport_dispatch_json fn
    tdj_fn_start = find_line(lines, "pub fn transport_dispatch_json(")
    tdj_fn_end = find_block_end(lines, tdj_fn_start)

    # nif_dispatch_json fn
    ndj_fn_start = find_line(lines, "fn nif_dispatch_json(")
    ndj_fn_end = find_block_end(lines, ndj_fn_start)

    # dispatch_transport_request fn
    dtr_fn_start = find_line(lines, "fn dispatch_transport_request(")
    dtr_fn_end = find_block_end(lines, dtr_fn_start)

    # dispatch_transport_action fn
    dta_fn_start = find_line(lines, "fn dispatch_transport_action(")
    dta_fn_end = find_block_end(lines, dta_fn_start)

    # NIF module
    nif_start = find_line(lines, '#[cfg(feature = "rustler_nif")]')
    nif_end = find_block_end(lines, find_line(lines, "mod nif", nif_start))

    # Tests module
    tests_start = find_line(lines, "#[cfg(test)]")
    tests_end = total  # goes to end of file

    # -----------------------------------------------------------------------
    # Step 2: Build module contents
    # -----------------------------------------------------------------------

    def g(start: int, end: int) -> str:
        return lines_text(lines, start, end)

    # === types.rs ===
    # Contains: LeaseActionKind, TransportActionId, TransportActionSpec,
    # REQUIRED_MORPHISMS_*, TRANSPORT_ACTION_SPECS, transport_action_spec fn,
    # WorldRouteBinding struct, world_binding fns, resolver fns,
    # semantic_digest, transport_dispatch_digest, transport_action_row_digest,
    # TransportActionRegistryRow, transport_action_row, transport_action_registry_rows,
    # LeaseInfo, IssueSummary, LeaseProjection, LeaseActionEnvelope,
    # LeaseMutationError, envelope helpers (accepted/rejected),
    # non_empty, TransportDispatchRequest, TransportWorldBindingRequest,
    # IssueClaimRequest/NextRequest/RenewRequest/ReleaseRequest,
    # InstructionRunRequest, FiberSpawnRequest, FiberJoinRequest, FiberCancelRequest,
    # transport_rejected, annotate_transport_dispatch_fields,
    # TransportCheckIssue, TransportCheckReport, TransportKernelBindingError

    types_rs = """\
use chrono::{DateTime, Duration, Utc};
use premath_bd::{
    AtomicStoreMutationError, ClaimNextError, DEFAULT_LEASE_TTL_SECONDS, Issue,
    IssueLease, IssueLeaseState, MAX_LEASE_TTL_SECONDS, MIN_LEASE_TTL_SECONDS, MemoryStore,
};
use premath_kernel::SiteResolveWitness;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

use crate::*;

"""

    # LeaseActionKind enum + impl
    types_rs += g(lease_action_kind_start, lease_action_kind_impl_end) + "\n"

    # TransportActionId enum + impl
    types_rs += g(transport_action_id_start, transport_action_id_impl_end) + "\n"

    # TransportActionSpec struct
    types_rs += g(transport_action_spec_start, transport_action_spec_end) + "\n\n"

    # REQUIRED_MORPHISMS constants
    types_rs += g(req_morph_start, req_morph_end) + "\n"

    # TRANSPORT_ACTION_SPECS array
    types_rs += g(specs_array_start, specs_array_end) + "\n\n"

    # transport_action_spec fn
    types_rs += make_pub_crate(g(tas_fn_start, tas_fn_end)) + "\n"

    # WorldRouteBinding struct
    types_rs += g(wrb_struct_start, wrb_struct_end) + "\n\n"

    # world_binding, world_binding_for_action, resolver_witness_ref,
    # resolve_witness_for_action, resolver_fields_for_action
    types_rs += make_pub_crate(g(world_binding_fn_start, resolver_fields_end)) + "\n"

    # semantic_digest fn
    types_rs += make_pub_crate(g(semantic_digest_start, semantic_digest_end)) + "\n"

    # transport_dispatch_digest fn
    types_rs += make_pub_crate(g(tdd_fn_start, tdd_fn_end)) + "\n"

    # transport_action_row_digest fn
    types_rs += make_pub_crate(g(tard_fn_start, tard_fn_end)) + "\n"

    # TransportActionRegistryRow struct
    types_rs += g(tarr_start, tarr_end) + "\n\n"

    # TransportCheckIssue struct
    types_rs += g(tci_start, tci_end) + "\n\n"

    # TransportCheckReport struct
    types_rs += g(tcr_start, tcr_end) + "\n\n"

    # transport_action_row fn
    types_rs += make_pub_crate(g(tar_fn_start, tar_fn_end)) + "\n"

    # transport_action_registry_rows fn (already pub)
    types_rs += g(tarr_fn_start, tarr_fn_end) + "\n"

    # transport_check_digest fn
    types_rs += make_pub_crate(g(tcd_fn_start, tcd_fn_end)) + "\n"

    # TransportKernelBindingError struct
    types_rs += make_pub_crate_struct(g(tkbe_start, tkbe_end)) + "\n\n"

    # resolve_site_for_spec fn
    types_rs += make_pub_crate(g(rsfs_fn_start, rsfs_fn_end)) + "\n"

    # LeaseInfo struct
    types_rs += g(li_start, li_end) + "\n\n"

    # IssueSummary struct
    types_rs += g(is_start, is_end) + "\n\n"

    # LeaseProjection struct
    types_rs += g(lp_start, lp_end) + "\n\n"

    # LeaseActionEnvelope struct
    types_rs += g(lae_start, lae_end) + "\n\n"

    # LeaseMutationError struct + impl (make pub(crate))
    types_rs += make_pub_crate_struct(g(lme_start, lme_impl_end)) + "\n"

    # accepted_envelope fn
    types_rs += make_pub_crate(g(ae_fn_start, ae_fn_end)) + "\n"

    # accepted_envelope_optional fn
    types_rs += make_pub_crate(g(aeo_fn_start, aeo_fn_end)) + "\n"

    # rejected_envelope fn
    types_rs += make_pub_crate(g(re_fn_start, re_fn_end)) + "\n"

    # map_atomic_store_error fn
    types_rs += make_pub_crate(g(mase_fn_start, mase_fn_end)) + "\n"

    # map_claim_next_atomic_store_error fn
    types_rs += make_pub_crate(g(mcnase_fn_start, mcnase_fn_end)) + "\n"

    # map_claim_next_error fn
    types_rs += make_pub_crate(g(mcne_fn_start, mcne_fn_end)) + "\n"

    # non_empty fn
    types_rs += make_pub_crate(g(ne_fn_start, ne_fn_end)) + "\n"

    # resolve_issues_path fn
    types_rs += make_pub_crate(g(rip_fn_start, rip_fn_end)) + "\n"

    # parse_lease_ttl_seconds fn
    types_rs += make_pub_crate(g(plts_fn_start, plts_fn_end)) + "\n"

    # parse_lease_expiry fn
    types_rs += make_pub_crate(g(ple_fn_start, ple_fn_end)) + "\n"

    # lease_token fn
    types_rs += make_pub_crate(g(lt_fn_start, lt_fn_end)) + "\n"

    # resolve_lease_id fn
    types_rs += make_pub_crate(g(rli_fn_start, rli_fn_end)) + "\n"

    # lease_state_label fn
    types_rs += make_pub_crate(g(lsl_fn_start, lsl_fn_end)) + "\n"

    # issue_is_lease_contended fn
    types_rs += make_pub_crate(g(iilc_fn_start, iilc_fn_end)) + "\n"

    # issue_summary fn
    types_rs += make_pub_crate(g(isf_fn_start, isf_fn_end)) + "\n"

    # compute_lease_projection fn
    types_rs += make_pub_crate(g(clp_fn_start, clp_fn_end)) + "\n"

    # transport_rejected fn
    types_rs += make_pub_crate(g(tr_fn_start, tr_fn_end)) + "\n"

    # annotate_transport_dispatch_fields fn
    types_rs += make_pub_crate(g(atdf_fn_start, atdf_fn_end)) + "\n"

    # Request types (all Deserialize structs)
    types_rs += g(icr_start, icr_end) + "\n\n"
    types_rs += g(icnr_start, icnr_end) + "\n\n"
    types_rs += g(ilrr_start, ilrr_end) + "\n\n"
    types_rs += g(ilrelr_start, ilrelr_end) + "\n\n"
    types_rs += g(tdr_start, tdr_end) + "\n\n"
    types_rs += g(twbr_start, twbr_end) + "\n\n"
    types_rs += g(irr_start, irr_end) + "\n\n"
    types_rs += g(fsr_start, fsr_end) + "\n\n"
    types_rs += g(fjr_start, fjr_end) + "\n\n"
    types_rs += g(fcr_start, fcr_end) + "\n"

    # === registry.rs ===
    # Contains: validate_transport_registry, validate_transport_action_binding_with_kernel,
    # transport_check

    registry_rs = """\
use premath_kernel::{
    RequiredRouteBinding, parse_operation_route_rows,
    validate_world_route_bindings_with_requirements, world_failure_class,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::types::*;
use crate::*;

"""

    registry_rs += make_pub_crate(g(vtr_fn_start, vtr_fn_end)) + "\n"
    registry_rs += make_pub_crate(g(vtabwk_fn_start, vtabwk_fn_end)) + "\n"
    registry_rs += g(tc_fn_start, tc_fn_end) + "\n"

    # === lease.rs ===
    # Contains: lease operations + JSON wrappers

    lease_rs = """\
use chrono::Utc;
use premath_bd::{
    ClaimNextRequest, MemoryStore,
    claim_next_issue_jsonl, mutate_store_jsonl,
};
use serde_json::Value;

use crate::registry::validate_transport_action_binding_with_kernel;
use crate::types::*;
use crate::*;

"""

    lease_rs += g(issue_claim_next_start, issue_claim_next_end) + "\n\n"
    lease_rs += g(issue_claim_start, issue_claim_end) + "\n\n"
    lease_rs += g(issue_lease_renew_start, issue_lease_renew_end) + "\n\n"
    lease_rs += g(issue_lease_release_start, issue_lease_release_end) + "\n\n"
    lease_rs += g(icj_start, icj_end) + "\n\n"
    lease_rs += g(icnj_start, icnj_end) + "\n\n"
    lease_rs += g(ilrj_start, ilrj_end) + "\n\n"
    lease_rs += g(ilrelj_start, ilrelj_end) + "\n"

    # === fiber.rs ===
    # Contains: fiber token/helpers + response functions

    fiber_rs = """\
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::types::*;
use crate::*;

"""

    fiber_rs += make_pub_crate(g(fiber_token_start, fiber_token_end)) + "\n"
    fiber_rs += make_pub_crate(g(derive_fiber_id_start, derive_fiber_id_end)) + "\n"
    fiber_rs += make_pub_crate(g(fiber_witness_ref_start, fiber_witness_ref_end)) + "\n"
    fiber_rs += make_pub_crate(g(fiber_rejected_start, fiber_rejected_end)) + "\n"
    fiber_rs += make_pub_crate(g(fiber_spawn_response_start, fiber_spawn_response_end)) + "\n"
    fiber_rs += make_pub_crate(g(fiber_join_response_start, fiber_join_response_end)) + "\n"
    fiber_rs += make_pub_crate(g(fiber_cancel_response_start, fiber_cancel_response_end)) + "\n"

    # === dispatch.rs ===
    # Contains: instruction helpers + dispatch orchestration + world_route_binding_json

    dispatch_rs = """\
use chrono::{DateTime, Utc};
use premath_coherence::{
    ExecutedInstructionCheck, InstructionError, InstructionWitness, InstructionWitnessRuntime,
    ValidatedInstructionEnvelope, build_instruction_witness, build_pre_execution_reject_witness,
    validate_instruction_envelope_payload,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::fiber::{fiber_spawn_response, fiber_join_response, fiber_cancel_response};
use crate::lease::{issue_claim, issue_claim_next, issue_lease_renew, issue_lease_release};
use crate::registry::validate_transport_action_binding_with_kernel;
use crate::types::*;
use crate::*;

"""

    # Instruction helpers
    dispatch_rs += make_pub_crate(g(truncate_start, truncate_end)) + "\n"
    dispatch_rs += make_pub_crate(g(resolve_repo_root_start, resolve_repo_root_end)) + "\n"
    dispatch_rs += make_pub_crate(g(resolve_instr_path_start, resolve_instr_path_end)) + "\n"
    dispatch_rs += make_pub_crate(g(instr_ref_start, instr_ref_end)) + "\n"
    dispatch_rs += make_pub_crate(g(fallback_instr_id_start, fallback_instr_id_end)) + "\n"
    dispatch_rs += make_pub_crate(g(instr_id_validated_start, instr_id_validated_end)) + "\n"
    dispatch_rs += make_pub_crate(g(sort_json_start, sort_json_end)) + "\n"
    dispatch_rs += make_pub_crate(g(norm_instr_digest_start, norm_instr_digest_end)) + "\n"
    dispatch_rs += make_pub_crate(g(instr_runtime_payload_start, instr_runtime_payload_end)) + "\n"
    dispatch_rs += make_pub_crate(g(write_instr_witness_start, write_instr_witness_end)) + "\n"
    dispatch_rs += make_pub_crate(g(run_gate_check_start, run_gate_check_end)) + "\n"
    dispatch_rs += make_pub_crate(g(decision_state_start, decision_state_end)) + "\n"
    dispatch_rs += make_pub_crate(g(decision_source_reason_start, decision_source_reason_end)) + "\n"
    dispatch_rs += make_pub_crate(g(instr_run_rejected_start, instr_run_rejected_end)) + "\n"
    dispatch_rs += make_pub_crate(g(instr_run_response_start, instr_run_response_end)) + "\n"

    # world_route_binding_json
    dispatch_rs += g(wrbj_fn_start, wrbj_fn_end) + "\n\n"

    # transport_dispatch_json
    dispatch_rs += g(tdj_fn_start, tdj_fn_end) + "\n\n"

    # nif_dispatch_json
    dispatch_rs += g(ndj_fn_start, ndj_fn_end) + "\n\n"

    # dispatch_transport_request fn
    dispatch_rs += make_pub_crate(g(dtr_fn_start, dtr_fn_end)) + "\n"

    # dispatch_transport_action fn
    dispatch_rs += make_pub_crate(g(dta_fn_start, dta_fn_end)) + "\n"

    # === lib.rs ===
    # Contains: imports, mod declarations, pub use re-exports, constants, NIF block, tests

    lib_rs = """\
use chrono::{DateTime, Duration, Utc};
use premath_bd::{
    AtomicStoreMutationError, ClaimNextError, ClaimNextRequest, DEFAULT_LEASE_TTL_SECONDS, Issue,
    IssueLease, IssueLeaseState, MAX_LEASE_TTL_SECONDS, MIN_LEASE_TTL_SECONDS, MemoryStore,
    claim_next_issue_jsonl, mutate_store_jsonl,
};
use premath_coherence::{
    ExecutedInstructionCheck, InstructionError, InstructionWitness, InstructionWitnessRuntime,
    ValidatedInstructionEnvelope, build_instruction_witness, build_pre_execution_reject_witness,
    validate_instruction_envelope_payload,
};
use premath_kernel::{
    RequiredRouteBinding, SiteResolveRequest, SiteResolveWitness, parse_operation_route_rows,
    resolve_site_request, validate_world_route_bindings_with_requirements, world_failure_class,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub mod types;
pub mod registry;
pub mod lease;
pub mod fiber;
pub mod dispatch;

// Re-export all public items from submodules so downstream callers are unaffected.
pub use types::*;
pub use registry::*;
pub use lease::*;
pub use fiber::*;
pub use dispatch::*;

"""

    # Constants
    lib_rs += g(const_start, const_end) + "\n"

    # NIF block
    lib_rs += g(nif_start, nif_end) + "\n\n"

    # Tests block
    lib_rs += g(tests_start, tests_end)

    return {
        "lib.rs": lib_rs,
        "types.rs": types_rs,
        "registry.rs": registry_rs,
        "lease.rs": lease_rs,
        "fiber.rs": fiber_rs,
        "dispatch.rs": dispatch_rs,
    }


def make_pub_crate(text: str) -> str:
    """Turn leading `fn ` into `pub(crate) fn ` for top-level private functions.
    Only converts `fn ` at the start of a line (possibly with leading whitespace),
    NOT inside match arms, closures, etc.
    Does not touch already-pub items."""
    result_lines = []
    for line in text.split("\n"):
        stripped = line.lstrip()
        # Only convert if it's a top-level fn declaration (no indentation or exactly 0 spaces)
        indent = len(line) - len(stripped)
        if indent == 0 and stripped.startswith("fn ") and not stripped.startswith("fn("):
            line = "pub(crate) " + line
        result_lines.append(line)
    return "\n".join(result_lines)


def make_pub_crate_struct(text: str) -> str:
    """Turn leading `struct ` into `pub(crate) struct ` and also its fields
    that don't already have a visibility qualifier. Also handles `fn ` in impl blocks."""
    result_lines = []
    in_struct_body = False
    brace_depth = 0
    for line in text.split("\n"):
        stripped = line.lstrip()
        indent = len(line) - len(stripped)

        # Convert top-level struct/fn
        if indent == 0 and stripped.startswith("struct "):
            line = "pub(crate) " + line
        elif indent == 0 and stripped.startswith("fn ") and not stripped.startswith("fn("):
            line = "pub(crate) " + line

        # Convert fields inside struct body: lines like `    failure_class: String,`
        # that don't start with `pub`
        if in_struct_body and brace_depth == 1 and indent > 0:
            if (not stripped.startswith("pub ")
                and not stripped.startswith("pub(")
                and not stripped.startswith("//")
                and not stripped.startswith("#[")
                and not stripped.startswith("}")
                and not stripped.startswith("{")
                and stripped
                and ":" in stripped):
                line = line[:indent] + "pub(crate) " + stripped

        # Track brace depth for struct bodies
        if "struct " in line and "{" in line:
            in_struct_body = True
            brace_depth = 0
        for ch in line:
            if ch == "{":
                brace_depth += 1
            elif ch == "}":
                brace_depth -= 1
                if brace_depth <= 0 and in_struct_body:
                    in_struct_body = False

        result_lines.append(line)
    return "\n".join(result_lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Split premath-transport lib.rs into modules")
    parser.add_argument("--write", action="store_true", help="Actually write files (default is dry-run)")
    args = parser.parse_args()

    if not LIB_RS.exists():
        print(f"ERROR: {LIB_RS} does not exist", file=sys.stderr)
        sys.exit(1)

    lines = read_lines(LIB_RS)
    print(f"Read {len(lines)} lines from {LIB_RS}")

    modules = build_modules(lines)

    for filename, content in sorted(modules.items()):
        line_count = content.count("\n")
        target = CRATE_ROOT / filename
        print(f"  {filename}: {line_count} lines -> {target}")

    if not args.write:
        print("\nDry run. Use --write to actually write files.")
        print("\nPreview of each file's first 5 lines:")
        for filename, content in sorted(modules.items()):
            print(f"\n--- {filename} ---")
            for line in content.split("\n")[:5]:
                print(f"  {line}")
        return

    # Back up original
    backup = LIB_RS.with_suffix(".rs.bak")
    if not backup.exists():
        import shutil
        shutil.copy2(LIB_RS, backup)
        print(f"\nBacked up original to {backup}")

    for filename, content in modules.items():
        target = CRATE_ROOT / filename
        with open(target, "w") as f:
            f.write(content)
        print(f"Wrote {target}")

    print("\nDone. Run `cargo check --package premath-transport` to verify.")


if __name__ == "__main__":
    main()
