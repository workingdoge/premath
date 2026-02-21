use crate::support::{
    CONFLICT_SAMPLE_LIMIT, fibers_or_exit, load_store_or_exit, maybe_jj_snapshot,
    parse_level_or_exit, print_sample_block, sample_with_truncation, scope_ids_or_exit,
};
use premath_kernel::{ContextId, DescentDatum};
use premath_surreal::QueryCache;
use serde_json::json;

pub fn run(id: String, level: String, issues: String, repo: String, json_output: bool) {
    let level = parse_level_or_exit(&level);
    let (store, issues_path) = load_store_or_exit(&issues);
    let cache = QueryCache::hydrate(&store);

    let scoped_ids = scope_ids_or_exit(&cache, &id);
    let context_id = ContextId::new(format!("scope:{id}"));
    let fibers = fibers_or_exit(&cache, &scoped_ids, &context_id);
    let datum = DescentDatum::assemble(format!("cover:{id}"), context_id.clone(), fibers, level);

    let glue_hash = datum.glue_hash().map(|h| h.0);
    let all_conflicts: Vec<String> = datum.conflicts().into_iter().map(str::to_string).collect();
    let (conflicts, conflicts_truncated) =
        sample_with_truncation(all_conflicts, CONFLICT_SAMPLE_LIMIT);
    let conflict_count = conflicts.len() + conflicts_truncated;

    if json_output {
        let payload = json!({
            "scope": id,
            "coherence_level": level.to_string(),
            "issues_path": issues_path.display().to_string(),
            "issue_count": scoped_ids.len(),
            "contractible": datum.is_effective(),
            "glue_hash": glue_hash,
            "conflict_count": conflict_count,
            "conflicts_sample": conflicts,
            "conflicts_truncated_count": conflicts_truncated,
            "jj_snapshot": maybe_jj_snapshot(&repo),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!("premath contract check {id} --level {level}");
        println!("  Source: {}", issues_path.display());
        println!("  Scope size: {}", scoped_ids.len());
        println!(
            "  Contractible: {}",
            if datum.is_effective() { "yes" } else { "no" }
        );
        if let Some(hash) = glue_hash {
            println!("  Glue hash: {hash}");
        }
        print_sample_block("Conflicts", &conflicts, conflicts_truncated);
    }
}
