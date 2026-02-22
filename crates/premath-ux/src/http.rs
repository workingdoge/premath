use crate::{ObserveQuery, ObserveQueryError, SurrealObservationBackend, UxService};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    pub bind: SocketAddr,
    pub surface: PathBuf,
}

#[derive(Debug, Error)]
pub enum HttpServeError {
    #[error("bind failed: {0}")]
    Bind(std::io::Error),
    #[error("accept failed: {0}")]
    Accept(std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpResponse {
    status: u16,
    body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Route {
    Healthz,
    Index,
    Latest,
    NeedsAttention,
    Instruction(String),
    Projection(String),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
enum RouteError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub fn serve_observation_api(config: HttpServerConfig) -> Result<(), HttpServeError> {
    serve_with_limit(config, None)
}

fn serve_with_limit(
    config: HttpServerConfig,
    max_requests: Option<usize>,
) -> Result<(), HttpServeError> {
    let listener = TcpListener::bind(config.bind).map_err(HttpServeError::Bind)?;
    let mut served = 0usize;

    for stream in listener.incoming() {
        if let Some(limit) = max_requests
            && served >= limit
        {
            break;
        }

        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_connection(&mut stream, &config.surface) {
                    let _ = write_json_response(
                        &mut stream,
                        HttpResponse {
                            status: 500,
                            body: json!({ "error": format!("internal server error: {err}") }),
                        },
                    );
                }
                served += 1;
            }
            Err(err) => return Err(HttpServeError::Accept(err)),
        }
    }

    Ok(())
}

fn handle_connection(stream: &mut TcpStream, surface: &PathBuf) -> Result<(), String> {
    let (method, target) = read_request_line(stream).map_err(|e| e.to_string())?;

    if method != "GET" {
        return write_json_response(
            stream,
            HttpResponse {
                status: 405,
                body: json!({ "error": "method not allowed; use GET" }),
            },
        )
        .map_err(|e| e.to_string());
    }

    let route = match parse_route_target(&target) {
        Ok(route) => route,
        Err(RouteError::BadRequest(msg)) => {
            return write_json_response(
                stream,
                HttpResponse {
                    status: 400,
                    body: json!({ "error": msg }),
                },
            )
            .map_err(|e| e.to_string());
        }
        Err(RouteError::NotFound(msg)) => {
            return write_json_response(
                stream,
                HttpResponse {
                    status: 404,
                    body: json!({ "error": msg }),
                },
            )
            .map_err(|e| e.to_string());
        }
    };

    let backend = SurrealObservationBackend::load_json(surface).map_err(|e| e.to_string())?;
    let service = UxService::new(backend);
    let response = execute_route(&service, route);
    write_json_response(stream, response).map_err(|e| e.to_string())
}

fn read_request_line(stream: &mut TcpStream) -> Result<(String, String), RouteError> {
    let mut buf = [0u8; 8192];
    let n = stream
        .read(&mut buf)
        .map_err(|e| RouteError::BadRequest(format!("failed to read request: {e}")))?;
    if n == 0 {
        return Err(RouteError::BadRequest("empty request".to_string()));
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let line = req
        .lines()
        .next()
        .ok_or_else(|| RouteError::BadRequest("missing request line".to_string()))?;
    parse_request_line(line)
}

fn parse_request_line(line: &str) -> Result<(String, String), RouteError> {
    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| RouteError::BadRequest("missing method".to_string()))?;
    let target = parts
        .next()
        .ok_or_else(|| RouteError::BadRequest("missing target".to_string()))?;
    Ok((method.to_string(), target.to_string()))
}

fn parse_route_target(target: &str) -> Result<Route, RouteError> {
    let (path, query) = split_target(target);
    let params = parse_query_params(query);

    match path {
        "/" => Ok(Route::Index),
        "/healthz" => Ok(Route::Healthz),
        "/latest" => Ok(Route::Latest),
        "/needs-attention" | "/needs_attention" => Ok(Route::NeedsAttention),
        "/instruction" => {
            let id = params
                .get("id")
                .or_else(|| params.get("instruction_id"))
                .cloned()
                .ok_or_else(|| {
                    RouteError::BadRequest(
                        "missing instruction id (use /instruction?id=<instruction_id>)".to_string(),
                    )
                })?;
            Ok(Route::Instruction(id))
        }
        "/projection" => {
            let digest = params
                .get("digest")
                .or_else(|| params.get("projection_digest"))
                .cloned()
                .ok_or_else(|| {
                    RouteError::BadRequest(
                        "missing projection digest (use /projection?digest=<projection_digest>)"
                            .to_string(),
                    )
                })?;
            Ok(Route::Projection(digest))
        }
        _ => Err(RouteError::NotFound(format!("unknown route: {path}"))),
    }
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

fn parse_query_params(query: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key = percent_decode(k);
        if key.is_empty() {
            continue;
        }
        out.insert(key, percent_decode(v));
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h * 16 + l) as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            ch => {
                out.push(ch as char);
                i += 1;
            }
        }
    }
    out
}

fn hex_val(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

fn execute_route<B>(service: &UxService<B>, route: Route) -> HttpResponse
where
    B: crate::ObservationBackend,
{
    match route {
        Route::Healthz => HttpResponse {
            status: 200,
            body: json!({ "ok": true }),
        },
        Route::Index => HttpResponse {
            status: 200,
            body: json!({
                "service": "premath.observe.v1",
                "routes": [
                    "/healthz",
                    "/latest",
                    "/needs-attention",
                    "/instruction?id=<instruction_id>",
                    "/projection?digest=<projection_digest>"
                ]
            }),
        },
        Route::Latest => match service.query_json(ObserveQuery::Latest) {
            Ok(body) => HttpResponse { status: 200, body },
            Err(err) => query_error_response(err),
        },
        Route::NeedsAttention => match service.query_json(ObserveQuery::NeedsAttention) {
            Ok(body) => HttpResponse { status: 200, body },
            Err(err) => query_error_response(err),
        },
        Route::Instruction(instruction_id) => {
            match service.query_json(ObserveQuery::Instruction { instruction_id }) {
                Ok(body) => HttpResponse { status: 200, body },
                Err(err) => query_error_response(err),
            }
        }
        Route::Projection(projection_digest) => {
            match service.query_json(ObserveQuery::Projection { projection_digest }) {
                Ok(body) => HttpResponse { status: 200, body },
                Err(err) => query_error_response(err),
            }
        }
    }
}

fn query_error_response(err: ObserveQueryError) -> HttpResponse {
    match err {
        ObserveQueryError::InstructionNotFound(msg)
        | ObserveQueryError::ProjectionNotFound(msg) => HttpResponse {
            status: 404,
            body: json!({ "error": msg }),
        },
        ObserveQueryError::Serialization(msg) => HttpResponse {
            status: 500,
            body: json!({ "error": msg }),
        },
    }
}

fn write_json_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    let body = serde_json::to_vec_pretty(&response.body)?;
    let status_text = reason_phrase(response.status);
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET\r\nConnection: close\r\n\r\n",
        response.status,
        status_text,
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DecisionSummary, DeltaSummary, InstructionSummary, ObservationBackend, ObservationSummary,
        ProjectionView, RequiredSummary,
    };

    #[derive(Clone)]
    struct MockBackend;

    impl ObservationBackend for MockBackend {
        fn summary(&self) -> ObservationSummary {
            ObservationSummary {
                state: "accepted".to_string(),
                needs_attention: false,
                top_failure_class: None,
                latest_projection_digest: Some("proj1_x".to_string()),
                latest_instruction_id: Some("i1".to_string()),
                required_check_count: 1,
                executed_check_count: 1,
                changed_path_count: 2,
                coherence: None,
            }
        }

        fn latest_delta(&self) -> Option<DeltaSummary> {
            None
        }

        fn latest_required(&self) -> Option<RequiredSummary> {
            Some(RequiredSummary {
                r#ref: "artifacts/ciwitness/latest-required.json".to_string(),
                witness_kind: Some("ci.required.v1".to_string()),
                projection_policy: Some("ci-topos-v0".to_string()),
                projection_digest: Some("proj1_x".to_string()),
                verdict_class: Some("accepted".to_string()),
                required_checks: vec!["baseline".to_string()],
                executed_checks: vec!["baseline".to_string()],
                failure_classes: vec![],
            })
        }

        fn latest_decision(&self) -> Option<DecisionSummary> {
            None
        }

        fn instruction(&self, instruction_id: &str) -> Option<InstructionSummary> {
            if instruction_id == "i1" {
                Some(InstructionSummary {
                    r#ref: "artifacts/ciwitness/i1.json".to_string(),
                    witness_kind: Some("ci.instruction.v1".to_string()),
                    instruction_id: "i1".to_string(),
                    instruction_digest: Some("instr1_x".to_string()),
                    instruction_classification: None,
                    intent: None,
                    scope: None,
                    policy_digest: None,
                    verdict_class: Some("accepted".to_string()),
                    required_checks: vec![],
                    executed_checks: vec![],
                    failure_classes: vec![],
                })
            } else {
                None
            }
        }

        fn projection(&self, projection_digest: &str) -> Option<ProjectionView> {
            if projection_digest == "proj1_x" {
                Some(ProjectionView {
                    projection_digest: "proj1_x".to_string(),
                    required: self.latest_required(),
                    delta: None,
                    decision: None,
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn route_parsing_handles_query_params() {
        let route = parse_route_target("/instruction?id=i1").expect("route should parse");
        assert_eq!(route, Route::Instruction("i1".to_string()));

        let route = parse_route_target("/projection?digest=proj1_x").expect("route should parse");
        assert_eq!(route, Route::Projection("proj1_x".to_string()));
    }

    #[test]
    fn route_parsing_reports_missing_params() {
        let err = parse_route_target("/instruction").expect_err("route should fail");
        assert!(matches!(err, RouteError::BadRequest(_)));
    }

    #[test]
    fn execute_route_maps_not_found_to_404() {
        let service = UxService::new(MockBackend);
        let response = execute_route(&service, Route::Instruction("missing".to_string()));
        assert_eq!(response.status, 404);
    }

    #[test]
    fn percent_decode_works_for_common_forms() {
        assert_eq!(percent_decode("needs%2Dattention"), "needs-attention");
        assert_eq!(percent_decode("i1+test"), "i1 test");
    }
}
