use premath_ux::http::{HttpServerConfig, serve_observation_api};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;

pub fn run(surface: String, bind: String) {
    let bind_addr: SocketAddr = bind.parse().unwrap_or_else(|e| {
        eprintln!("error: invalid --bind address `{bind}`: {e}");
        process::exit(1);
    });

    let config = HttpServerConfig {
        bind: bind_addr,
        surface: PathBuf::from(&surface),
    };

    println!("premath observe-serve");
    println!("  bind: {}", bind_addr);
    println!("  surface: {}", surface);
    println!("  routes:");
    println!("    GET /healthz");
    println!("    GET /latest");
    println!("    GET /needs-attention");
    println!("    GET /instruction?id=<instruction_id>");
    println!("    GET /projection?digest=<projection_digest>[&match=typed|compatibility_alias]");

    if let Err(e) = serve_observation_api(config) {
        eprintln!("error: observation API failed: {e}");
        process::exit(1);
    }
}
