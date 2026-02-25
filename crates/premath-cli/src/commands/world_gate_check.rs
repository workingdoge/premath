use crate::support::read_json_file_or_exit;
use premath_kernel::gate::run_gate_check;
use premath_kernel::{GateCheck, GateResult, OperationRouteRow, World, parse_operation_route_rows};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const WORLD_GATE_CHECK_KIND: &str = "premath.world_gate_check.v1";

pub fn run(operations: String, check: String, profile: String, json_output: bool) {
    let operations_rows = load_operation_rows_or_exit(&operations);
    let world = OperationRegistryWorld::new(operations_rows).unwrap_or_else(|err| {
        eprintln!("error: failed to construct operation world: {err}");
        std::process::exit(1);
    });
    let gate_check = load_gate_check_or_exit(&check);
    let result = run_gate_check(&world, &gate_check, &profile);
    emit_result(result, &operations, &check, &profile, json_output);
}

fn load_gate_check_or_exit(path: &str) -> GateCheck {
    let raw: Value = read_json_file_or_exit(path, "gate check");
    let candidate = raw.get("check").unwrap_or(&raw);
    GateCheck::from_fixture(candidate).unwrap_or_else(|| {
        eprintln!(
            "error: failed to parse gate check at {} (expected fixture-compatible object)",
            path
        );
        std::process::exit(1);
    })
}

fn load_operation_rows_or_exit(path: &str) -> Vec<OperationRouteRow> {
    let raw = read_json_file_or_exit(path, "operation route rows");
    parse_operation_route_rows(&raw).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to parse operation route rows at {}: {}",
            path, err
        );
        std::process::exit(1);
    })
}

fn emit_result(
    result: GateResult,
    operations_path: &str,
    check_path: &str,
    profile: &str,
    json_output: bool,
) {
    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": WORLD_GATE_CHECK_KIND,
            "profile": profile,
            "operationsPath": operations_path,
            "checkPath": check_path,
            "result": result,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render world-gate-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }
    println!("premath world-gate-check");
    println!("  Profile: {profile}");
    println!("  Operations path: {operations_path}");
    println!("  Check path: {check_path}");
    println!("  Result: {}", result.result);
    println!("  Failures: {}", result.failures.len());
}

#[derive(Debug, Clone)]
struct OperationRegistryWorld {
    name: String,
    op_masks: BTreeMap<String, u64>,
}

impl OperationRegistryWorld {
    fn new(rows: Vec<OperationRouteRow>) -> Result<Self, String> {
        if rows.is_empty() {
            return Err("operation registry must be non-empty".to_string());
        }
        let mut morphism_bit_index: BTreeMap<String, usize> = BTreeMap::new();
        for row in &rows {
            for morphism in &row.morphisms {
                let key = morphism.trim().to_string();
                if key.is_empty() {
                    continue;
                }
                let next_index = morphism_bit_index.len();
                morphism_bit_index.entry(key).or_insert(next_index);
            }
        }
        if morphism_bit_index.len() > 64 {
            return Err(format!(
                "operation world requires <=64 distinct morphisms, found {}",
                morphism_bit_index.len()
            ));
        }

        let mut op_masks = BTreeMap::new();
        for row in rows {
            if row.operation_id.trim().is_empty() {
                continue;
            }
            let mut mask = 0u64;
            for morphism in row.morphisms {
                let key = morphism.trim();
                if key.is_empty() {
                    continue;
                }
                if let Some(index) = morphism_bit_index.get(key) {
                    mask |= 1u64 << index;
                }
            }
            op_masks.insert(row.operation_id.trim().to_string(), mask);
        }
        if op_masks.is_empty() {
            return Err("operation registry does not contain valid operation IDs".to_string());
        }

        Ok(Self {
            name: "control_plane_operation_world".to_string(),
            op_masks,
        })
    }

    fn parse_operation_id<'a>(&self, value: &'a Value) -> Option<&'a str> {
        value.as_str().map(str::trim).filter(|id| !id.is_empty())
    }

    fn operation_mask(&self, operation_id: &str) -> Option<u64> {
        self.op_masks.get(operation_id).copied()
    }
}

impl World for OperationRegistryWorld {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_definable(&self, gamma: u64, a: &Value) -> bool {
        let Some(operation_id) = self.parse_operation_id(a) else {
            return false;
        };
        let Some(mask) = self.operation_mask(operation_id) else {
            return false;
        };
        (mask & gamma) == gamma
    }

    fn restrict(&self, a: &Value, src: u64, tgt: u64) -> Option<Value> {
        if (tgt & !src) != 0 {
            return None;
        }
        let operation_id = self.parse_operation_id(a)?;
        let mask = self.operation_mask(operation_id)?;
        if (mask & src) != src || (mask & tgt) != tgt {
            return None;
        }
        Some(Value::String(operation_id.to_string()))
    }

    fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
        a == b
    }

    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool {
        let union = legs.iter().fold(0u64, |acc, leg| acc | *leg);
        (union & gamma) == gamma
    }

    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
        leg_i & leg_j
    }

    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        let mut out = Vec::new();
        for (operation_id, mask) in &self.op_masks {
            if (mask & gamma) == gamma {
                out.push(Value::String(operation_id.clone()));
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use premath_kernel::witness::failure_class;

    fn sample_operations() -> Vec<OperationRouteRow> {
        vec![
            OperationRouteRow {
                operation_id: "op/ci.run_gate".to_string(),
                morphisms: vec![
                    "dm.identity".to_string(),
                    "dm.profile.execution".to_string(),
                ],
            },
            OperationRouteRow {
                operation_id: "op/ci.run_instruction".to_string(),
                morphisms: vec![
                    "dm.identity".to_string(),
                    "dm.profile.execution".to_string(),
                    "dm.commitment.attest".to_string(),
                ],
            },
        ]
    }

    #[test]
    fn locality_accepts_when_operation_supports_cover_legs() {
        let world = OperationRegistryWorld::new(sample_operations()).expect("world");
        let check = GateCheck::Locality {
            gamma_mask: 0b11,
            a: Value::String("op/ci.run_gate".to_string()),
            legs: vec![0b01, 0b10],
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "control-plane");
        assert!(result.is_accepted());
    }

    #[test]
    fn locality_rejects_when_operation_lacks_required_leg() {
        let world = OperationRegistryWorld::new(sample_operations()).expect("world");
        let check = GateCheck::Locality {
            gamma_mask: 0b100,
            a: Value::String("op/ci.run_gate".to_string()),
            legs: vec![0b100],
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "control-plane");
        assert!(!result.is_accepted());
        assert_eq!(result.failures[0].class, failure_class::LOCALITY_FAILURE);
    }

    #[test]
    fn descent_accepts_unique_glue() {
        let world = OperationRegistryWorld::new(sample_operations()).expect("world");
        let check = GateCheck::Descent {
            base_mask: 0b1,
            legs: vec![0b1, 0b1],
            locals: vec![
                Value::String("op/ci.run_gate".to_string()),
                Value::String("op/ci.run_gate".to_string()),
            ],
            token_path: None,
            glue: None,
            overlap_certified: false,
            cocycle_certified: false,
            contractible_certified: false,
            contractible_scheme_id: None,
            contractible_proof: None,
        };
        let result = run_gate_check(&world, &check, "control-plane");
        assert!(result.is_accepted());
    }

    #[test]
    fn helper_parse_operations_supports_registry_object() {
        let raw = json!({
            "operations": [
                {"id": "op/ci.run_gate", "morphisms": ["dm.identity"]}
            ],
        });
        let rows = parse_operation_route_rows(&raw).expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].operation_id, "op/ci.run_gate");
    }
}
