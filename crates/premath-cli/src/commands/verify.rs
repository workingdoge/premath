use crate::support::{
    CONFLICT_SAMPLE_LIMIT, fibers_or_exit, load_store_or_exit, maybe_jj_snapshot,
    parse_level_or_exit, print_sample_block, sample_with_truncation, scope_ids_or_exit, yes_no,
};
use premath_kernel::{ContextId, DescentDatum, check_refinement_invariance};
use serde_json::json;

pub fn run(id: String, level: String, issues: String, repo: String, json_output: bool) {
    let level = parse_level_or_exit(&level);
    let (store, issues_path) = load_store_or_exit(&issues);
    let cache = premath_surreal::QueryCache::hydrate(&store);

    let scoped_ids = scope_ids_or_exit(&cache, &id);
    let context_id = ContextId::new(format!("scope:{id}"));
    let fibers = fibers_or_exit(&cache, &scoped_ids, &context_id);

    let datum = DescentDatum::assemble(
        format!("cover:{id}"),
        context_id.clone(),
        fibers.clone(),
        level,
    );
    let refined = DescentDatum::assemble(format!("refined-cover:{id}"), context_id, fibers, level);

    let locality_violations =
        premath_kernel::descent::detect_locality_violations(0, &scoped_ids, &|issue_id| {
            store
                .blocking_dependencies_of(issue_id)
                .into_iter()
                .map(|dep| dep.depends_on_id.clone())
                .collect()
        });
    let (locality_sample, locality_truncated) =
        sample_with_truncation(locality_violations, CONFLICT_SAMPLE_LIMIT);
    let locality_count = locality_sample.len() + locality_truncated;
    let locality_ok = locality_count == 0;
    let locality_lines: Vec<String> = locality_sample
        .iter()
        .map(|violation| violation.description.clone())
        .collect();

    let gluing_ok = datum.is_effective();
    let uniqueness_ok = datum.glue_hash().is_some();
    let refinement_ok = check_refinement_invariance(&datum, &refined).is_ok();
    let all_conflicts: Vec<String> = datum.conflicts().into_iter().map(str::to_string).collect();
    let (conflict_sample, conflicts_truncated) =
        sample_with_truncation(all_conflicts, CONFLICT_SAMPLE_LIMIT);
    let conflict_count = conflict_sample.len() + conflicts_truncated;

    if json_output {
        let payload = json!({
            "scope": id,
            "coherence_level": level.to_string(),
            "issues_path": issues_path.display().to_string(),
            "issue_count": scoped_ids.len(),
            "axioms": {
                "stability": "backend-dependent (not checked in static JSONL mode)",
                "locality": locality_ok,
                "gluing": gluing_ok,
                "uniqueness": uniqueness_ok,
                "refinement": refinement_ok,
            },
            "violations": {
                "locality_sample": locality_sample,
                "locality_count": locality_count,
                "locality_truncated_count": locality_truncated,
                "descent_conflicts_sample": conflict_sample,
                "descent_conflict_count": conflict_count,
                "descent_conflicts_truncated_count": conflicts_truncated,
            },
            "jj_snapshot": maybe_jj_snapshot(&repo),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!("premath axiom verify {id} --level {level}");
        println!("  Source: {}", issues_path.display());
        println!("  Scope size: {}", scoped_ids.len());
        println!("  Stability: backend-dependent (not checked in static JSONL mode)");
        println!("  Locality: {}", yes_no(locality_ok));
        println!("  Gluing: {}", yes_no(gluing_ok));
        println!("  Uniqueness: {}", yes_no(uniqueness_ok));
        println!("  Refinement: {}", yes_no(refinement_ok));
        print_sample_block("Locality violations", &locality_lines, locality_truncated);
        print_sample_block("Descent conflicts", &conflict_sample, conflicts_truncated);
    }
}
