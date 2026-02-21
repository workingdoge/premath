//! Integration tests: run the spec's toy test vectors.
//!
//! Each fixture in tests/fixtures/ has:
//! - case.json: the gate check input
//! - expect.json: the expected gate result
//!
//! These tests load the fixtures, parse the check, run the gate, and
//! compare the output to the expected result — including exact witness IDs.

use premath_kernel::gate::{GateCheck, run_gate_check};
use premath_kernel::toy::get_world;
use serde_json::Value;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn run_fixture(name: &str) {
    let dir = fixtures_dir().join(name);

    let case_path = dir.join("case.json");
    let expect_path = dir.join("expect.json");

    let case_str = std::fs::read_to_string(&case_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", case_path.display()));
    let expect_str = std::fs::read_to_string(&expect_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", expect_path.display()));

    let case: Value = serde_json::from_str(&case_str)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", case_path.display()));
    let expected: Value = serde_json::from_str(&expect_str)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", expect_path.display()));

    // Get the world
    let world_name = case["world"].as_str().expect("missing world field");
    let world = get_world(world_name).unwrap_or_else(|| panic!("unknown world: {world_name}"));

    // Parse the check
    let check = GateCheck::from_fixture(&case["check"])
        .unwrap_or_else(|| panic!("failed to parse check from {}", case_path.display()));

    // Run the gate
    let result = run_gate_check(world.as_ref(), &check, "toy");
    let result_json = serde_json::to_value(&result).expect("failed to serialize result");

    // Compare
    assert_eq!(
        result_json,
        expected,
        "\n\nFixture: {name}\n\nGot:\n{}\n\nExpected:\n{}\n",
        serde_json::to_string_pretty(&result_json).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap(),
    );
}

#[test]
fn golden_stability_sheaf_bits() {
    run_fixture("golden_stability_sheaf_bits");
}

#[test]
fn golden_descent_sheaf_bits() {
    run_fixture("golden_descent_sheaf_bits");
}

#[test]
fn adversarial_stability_failure_bad_stability() {
    run_fixture("adversarial_stability_failure_bad_stability");
}

#[test]
fn adversarial_locality_failure_partial_restrict() {
    run_fixture("adversarial_locality_failure_partial_restrict");
}

#[test]
fn adversarial_descent_failure_bad_constant() {
    run_fixture("adversarial_descent_failure_bad_constant");
}

#[test]
fn adversarial_glue_non_contractible_non_separated() {
    run_fixture("adversarial_glue_non_contractible_non_separated");
}

#[test]
fn golden_descent_sheaf_bits_cocycle() {
    run_fixture("golden_descent_sheaf_bits_cocycle");
}
