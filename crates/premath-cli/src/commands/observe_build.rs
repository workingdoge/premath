use premath_surreal::{build_events, build_surface};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

pub struct Args {
    pub repo_root: String,
    pub ciwitness_dir: String,
    pub issues_path: String,
    pub out_json: String,
    pub out_jsonl: String,
    pub json: bool,
}

pub fn run(args: Args) {
    let repo_root = resolve_repo_root(&args.repo_root);
    let ciwitness_dir = resolve_rel_path(&repo_root, &args.ciwitness_dir);
    let issues_path = resolve_rel_path(&repo_root, &args.issues_path);
    let out_json = resolve_rel_path(&repo_root, &args.out_json);
    let out_jsonl = resolve_rel_path(&repo_root, &args.out_jsonl);

    let surface = build_surface(&repo_root, &ciwitness_dir, Some(&issues_path)).unwrap_or_else(|e| {
        eprintln!(
            "error: failed to build observation surface (repoRoot={}, ciwitness={}, issues={}): {e}",
            repo_root.display(),
            ciwitness_dir.display(),
            issues_path.display()
        );
        process::exit(1);
    });

    if let Some(parent) = out_json.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!(
            "error: failed to create output directory {}: {e}",
            parent.display()
        );
        process::exit(1);
    }
    let surface_json = serde_json::to_string_pretty(&surface).unwrap_or_else(|e| {
        eprintln!("error: failed to render observation surface json: {e}");
        process::exit(1);
    });
    if let Err(e) = fs::write(&out_json, format!("{surface_json}\n")) {
        eprintln!(
            "error: failed to write observation surface {}: {e}",
            out_json.display()
        );
        process::exit(1);
    }

    if let Some(parent) = out_jsonl.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!(
            "error: failed to create events output directory {}: {e}",
            parent.display()
        );
        process::exit(1);
    }
    let events = build_events(&surface);
    let mut jsonl = String::new();
    for event in events {
        let line = serde_json::to_string(&event).unwrap_or_else(|e| {
            eprintln!("error: failed to render observation event json: {e}");
            process::exit(1);
        });
        jsonl.push_str(&line);
        jsonl.push('\n');
    }
    if let Err(e) = fs::write(&out_jsonl, jsonl) {
        eprintln!(
            "error: failed to write observation events {}: {e}",
            out_jsonl.display()
        );
        process::exit(1);
    }

    if args.json {
        println!("{surface_json}");
        return;
    }

    let attention_reason_count = surface
        .summary
        .coherence
        .as_ref()
        .and_then(|coherence| coherence.get("attentionReasons"))
        .and_then(|value| value.as_array())
        .map_or(0, |items| items.len());
    println!(
        "[observation-surface] OK (state={}, needsAttention={}, attentionReasons={}, out={})",
        surface.summary.state,
        surface.summary.needs_attention,
        attention_reason_count,
        out_json.display()
    );
}

fn resolve_repo_root(input: &str) -> PathBuf {
    let path = PathBuf::from(input.trim());
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn resolve_rel_path(root: &Path, input: &str) -> PathBuf {
    let path = PathBuf::from(input.trim());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}
