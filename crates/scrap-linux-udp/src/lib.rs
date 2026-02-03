use scrap_core_lite::{
    decode_envelope, encode_envelope, Envelope, RouteEntry, RouteTable, MSG_TASK_REJECTED,
};
use scrap_edge::{
    handle_envelope, Action, Context, ReplayCache, TokenVerifier, DETAIL_SUBJECT_MISMATCH,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::UdpSocket;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct RoutesFile {
    pub nodes: HashMap<String, NodeRoutes>,
}

#[derive(Debug, Deserialize)]
pub struct NodeRoutes {
    pub routes: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_id: String,
    pub bind: String,
    pub port: u16,
    pub routes_path: String,
    pub commander_pubkey: Option<String>,
    pub replay_cache_path: String,
    pub revoked_path: String,
    pub allow_mock_signatures: bool,
}

#[derive(Debug, Deserialize)]
struct NodeConfigFile {
    node_id: Option<String>,
    bind: Option<String>,
    port: Option<u16>,
    routes_path: Option<String>,
    commander_pubkey: Option<String>,
    replay_cache_path: Option<String>,
    revoked_path: Option<String>,
    allow_mock_signatures: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub node_id: String,
    pub bind: String,
    pub port: u16,
    pub routes_path: String,
}

pub struct DevTokenVerifier {
    pub allow_mock_signatures: bool,
}

impl TokenVerifier for DevTokenVerifier {
    fn verify(
        &self,
        token: &scrap_core_lite::Token,
        commander_pubkey: &str,
        expected_audience: &str,
        required_capability: &str,
        now: u64,
        revoked: &[Vec<u8>],
        expected_commander_pubkey: Option<&str>,
    ) -> Result<(), Vec<String>> {
        let mut details = Vec::new();

        if !self.allow_mock_signatures {
            details.push("signature verification not implemented".to_string());
        }

        if token.expires_at < now {
            details.push("token expired".to_string());
        }

        if token.subject != commander_pubkey {
            details.push(DETAIL_SUBJECT_MISMATCH.to_string());
        }

        if let Some(expected) = expected_commander_pubkey {
            if commander_pubkey != expected {
                details.push("commander_pubkey not authorized".to_string());
            }
        }

        if token.audience != expected_audience {
            details.push("token audience mismatch".to_string());
        }

        if token.capability != required_capability {
            details.push("token capability mismatch".to_string());
        }

        if revoked
            .iter()
            .any(|token_id| token_id.as_slice() == token.token_id.as_slice())
        {
            details.push("token revoked".to_string());
        }

        if details.is_empty() {
            Ok(())
        } else {
            Err(details)
        }
    }
}

pub struct FileReplayCache {
    path: String,
}

impl FileReplayCache {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    fn lock_path(&self) -> String {
        format!("{}.lock", self.path)
    }

    fn with_lock<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce() -> Option<R>,
    {
        let lock_path = self.lock_path();
        for _ in 0..200 {
            match OpenOptions::new().write(true).create_new(true).open(&lock_path) {
                Ok(_) => {
                    let result = f();
                    let _ = fs::remove_file(&lock_path);
                    return result;
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(25));
                }
            }
        }
        None
    }

    fn ensure_parent(path: &str) {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
    }

    fn load_list(&self) -> Vec<String> {
        if let Ok(raw) = fs::read_to_string(&self.path) {
            if let Ok(list) = serde_json::from_str::<Vec<String>>(&raw) {
                return list;
            }
        }
        Vec::new()
    }

    fn save_list(&self, list: &[String]) -> bool {
        Self::ensure_parent(&self.path);
        let tmp_path = format!("{}.tmp", self.path);
        if let Ok(payload) = serde_json::to_vec_pretty(list) {
            if let Ok(mut file) = OpenOptions::new().write(true).create(true).truncate(true).open(&tmp_path) {
                if file.write_all(&payload).is_ok() {
                    let _ = fs::rename(&tmp_path, &self.path);
                    return true;
                }
            }
        }
        false
    }
}

impl ReplayCache for FileReplayCache {
    fn check_and_add(&mut self, token_id: &[u8]) -> bool {
        let token_hex = hex_encode(token_id);
        self.with_lock(|| {
            let mut list = self.load_list();
            if list.iter().any(|item| item == &token_hex) {
                return Some(false);
            }
            list.push(token_hex);
            Some(self.save_list(&list))
        })
        .unwrap_or(false)
    }
}

pub fn load_routes(path: &str, node_id: &str) -> Result<RouteTable, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("routes read failed: {e}"))?;
    let routes_file: RoutesFile = serde_json::from_str(&raw).map_err(|e| format!("routes parse failed: {e}"))?;
    let node_routes = routes_file
        .nodes
        .get(node_id)
        .ok_or_else(|| format!("routes missing node_id={node_id}"))?;
    let mut entries = Vec::new();
    for (dst, next_hop) in &node_routes.routes {
        entries.push(RouteEntry {
            dst: dst.clone(),
            next_hop: next_hop.clone(),
        });
    }
    Ok(RouteTable::new(entries))
}

pub fn load_node_config(path: &str) -> Result<NodeConfig, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("config read failed: {e}"))?;
    let cfg: NodeConfigFile =
        serde_json::from_str(&raw).map_err(|e| format!("config parse failed: {e}"))?;

    let node_id = cfg
        .node_id
        .ok_or_else(|| "config missing node_id".to_string())?;

    Ok(NodeConfig {
        node_id,
        bind: cfg.bind.unwrap_or_else(|| "0.0.0.0".to_string()),
        port: cfg.port.unwrap_or(7227),
        routes_path: cfg
            .routes_path
            .unwrap_or_else(|| "inventory/routes.json".to_string()),
        commander_pubkey: cfg.commander_pubkey,
        replay_cache_path: cfg
            .replay_cache_path
            .unwrap_or_else(|| "demo/runtime/replay_cache.json".to_string()),
        revoked_path: cfg
            .revoked_path
            .unwrap_or_else(|| "demo/config/revoked.json".to_string()),
        allow_mock_signatures: cfg.allow_mock_signatures.unwrap_or(false),
    })
}

pub fn load_revoked(path: &str) -> Vec<Vec<u8>> {
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&raw) {
            return list
                .iter()
                .filter_map(|item| hex_decode(item))
                .collect::<Vec<_>>();
        }
    }
    Vec::new()
}

pub fn run_node(config: NodeConfig) -> Result<(), String> {
    let routes = load_routes(&config.routes_path, &config.node_id)?;
    let revoked = load_revoked(&config.revoked_path);
    let mut replay_cache = FileReplayCache::new(config.replay_cache_path.clone());
    let verifier = DevTokenVerifier {
        allow_mock_signatures: config.allow_mock_signatures,
    };

    let bind_addr = format!("{}:{}", config.bind, config.port);
    let socket = UdpSocket::bind(&bind_addr).map_err(|e| format!("bind failed: {e}"))?;

    log_json("executor_started", serde_json::json!({
        "bind": config.bind,
        "port": config.port,
        "node_id": config.node_id,
        "allow_mock_signatures": config.allow_mock_signatures
    }));

    let mut buf = [0u8; 2048];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).map_err(|e| format!("recv failed: {e}"))?;
        let env = match decode_envelope(&buf[..len]) {
            Ok(env) => env,
            Err(err) => {
                log_json(
                    "invalid_cbor",
                    serde_json::json!({
                        "error": format!("{err}"),
                        "source": addr.to_string(),
                        "len": len,
                        "preview": hex_preview(&buf[..len], 32)
                    }),
                );
                continue;
            }
        };

        let mut ctx = Context {
            node_id: &config.node_id,
            routes: &routes,
            replay: &mut replay_cache,
            revoked: &revoked,
            commander_pubkey: config.commander_pubkey.as_deref(),
            allow_mock_signatures: config.allow_mock_signatures,
            verifier: &verifier,
        };

        let now = unix_ts();
        match handle_envelope(&mut ctx, env, now) {
            Action::Forward { next_hop, envelope } => {
                if let Ok(payload) = encode_to_vec(&envelope) {
                    let _ = socket.send_to(&payload, next_hop);
                }
            }
            Action::Reply { envelope } => {
                if envelope.msg_type == MSG_TASK_REJECTED {
                    log_json("task_rejected", serde_json::json!({
                        "trace_id": hex_encode(&envelope.trace_id),
                        "dst": envelope.dst,
                        "reason": "validation_failed"
                    }));
                }
                if let Ok(payload) = encode_to_vec(&envelope) {
                    if let Some(next_hop) = routes.next_hop(&envelope.dst) {
                        let _ = socket.send_to(&payload, next_hop);
                    }
                }
            }
            Action::Execute { task, envelope } => {
                log_json("task_accepted", serde_json::json!({
                    "trace_id": hex_encode(&envelope.trace_id),
                    "command": task.command,
                    "dst": envelope.dst
                }));
                let start = std::time::Instant::now();
                let (status, output_digest) = execute_stub(&task.command, &task.args);
                let duration_ms = start.elapsed().as_millis() as u32;
                let result = scrap_edge::build_result_envelope(
                    envelope.trace_id.clone(),
                    config.node_id.clone(),
                    task.reply_to.clone(),
                    envelope.hop_limit,
                    status,
                    output_digest,
                    duration_ms,
                );

                if let Ok(payload) = encode_to_vec(&result) {
                    if let Some(next_hop) = routes.next_hop(&result.dst) {
                        let _ = socket.send_to(&payload, next_hop);
                    }
                }

                log_json("proof_sent", serde_json::json!({
                    "trace_id": hex_encode(&result.trace_id),
                    "dst": result.dst,
                    "duration_ms": duration_ms
                }));
            }
            Action::Drop => {}
        }
    }
}

pub fn execute_stub(command: &str, args: &str) -> (u8, Vec<u8>) {
    match command {
        "demo.hash" => {
            let val = args.parse::<u64>().unwrap_or(0);
            (0, scrap_edge::simple_digest(val))
        }
        "demo.sleep" => {
            let val = args.parse::<u64>().unwrap_or(0);
            let dur = Duration::from_millis(val.min(5000));
            thread::sleep(dur);
            (0, scrap_edge::simple_digest(val))
        }
        _ => (1, scrap_edge::simple_digest(0)),
    }
}

fn encode_to_vec(env: &Envelope) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    encode_envelope(env, &mut buf).map_err(|e| format!("encode failed: {:?}", e))?;
    Ok(buf)
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn log_json(event: &str, payload: serde_json::Value) {
    let log = serde_json::json!({
        "ts": unix_ts(),
        "event": event,
        "details": payload
    });
    println!("{}", log);
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

pub fn hex_preview(bytes: &[u8], max: usize) -> String {
    let take = core::cmp::min(bytes.len(), max);
    let mut out = String::with_capacity(take * 2);
    for b in &bytes[..take] {
        out.push_str(&format!("{:02x}", b));
    }
    if bytes.len() > max {
        out.push_str("..");
    }
    out
}

pub fn hex_decode(value: &str) -> Option<Vec<u8>> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let hi = decode_nibble(bytes[idx])?;
        let lo = decode_nibble(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
        idx += 2;
    }
    Some(out)
}

fn decode_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        b'A'..=b'F' => Some(10 + (b - b'A')),
        _ => None,
    }
}
