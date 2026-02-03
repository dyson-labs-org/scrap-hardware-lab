use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::UdpSocket;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(name = "scrap-executor", about = "SCRAP executor (Rust demo)")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    #[arg(long, default_value_t = 7227)]
    port: u16,

    #[arg(long)]
    policy: String,

    #[arg(long)]
    keys: String,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    allow_mock_signatures: bool,
}

#[derive(Debug, Deserialize)]
struct Policy {
    node_id: Option<String>,
    replay_cache_path: Option<String>,
    revocation_list_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Keys {
    commander_pubkey: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Token {
    version: u8,
    token_id: String,
    subject: String,
    audience: String,
    capability: String,
    issued_at: u64,
    expires_at: u64,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct TaskRequest {
    version: u8,
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    requested_capability: String,
    token: Token,
    commander_pubkey: String,
    commander_signature: String,
}

#[derive(Debug, Serialize)]
struct TaskAccepted {
    version: u8,
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    payment_hash: String,
}

#[derive(Debug, Serialize)]
struct Proof {
    version: u8,
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    proof_hash: String,
}

#[derive(Debug, Serialize)]
struct TaskRejected {
    version: u8,
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    details: Vec<String>,
    notes: Vec<String>,
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn sha256_hex(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
    }
    to_hex(&hasher.finalize())
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &str) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn ensure_parent(path: &str) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}

fn with_lock<F, R>(path: &str, f: F) -> Option<R>
where
    F: FnOnce() -> Option<R>,
{
    let lock_path = format!("{}.lock", path);
    for _ in 0..200 {
        match OpenOptions::new().write(true).create_new(true).open(&lock_path) {
            Ok(_) => {
                let result = f();
                let _ = fs::remove_file(&lock_path);
                return result;
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        }
    }
    None
}

fn load_string_list(path: &str) -> Vec<String> {
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&raw) {
            return list;
        }
    }
    Vec::new()
}

fn save_string_list(path: &str, list: &[String]) -> bool {
    ensure_parent(path);
    let tmp_path = format!("{}.tmp", path);
    if let Ok(payload) = serde_json::to_vec_pretty(list) {
        if let Ok(mut file) = OpenOptions::new().write(true).create(true).truncate(true).open(&tmp_path) {
            if file.write_all(&payload).is_ok() {
                let _ = fs::rename(&tmp_path, path);
                return true;
            }
        }
    }
    false
}

fn replay_check_and_add(path: &str, token_id: &str) -> Option<bool> {
    with_lock(path, || {
        let mut list = load_string_list(path);
        if list.iter().any(|t| t == token_id) {
            return Some(false);
        }
        list.push(token_id.to_string());
        if save_string_list(path, &list) {
            Some(true)
        } else {
            None
        }
    })
}

fn main() {
    let args = Args::parse();

    let policy: Policy = read_json_file(&args.policy).unwrap_or(Policy {
        node_id: None,
        replay_cache_path: None,
        revocation_list_path: None,
    });
    let keys: Keys = read_json_file(&args.keys).unwrap_or(Keys {
        commander_pubkey: None,
    });

    let node_id = policy
        .node_id
        .unwrap_or_else(|| "JETSON-A".to_string());
    let replay_cache_path = policy
        .replay_cache_path
        .unwrap_or_else(|| "demo/runtime/replay_cache.json".to_string());
    let revoked_path = policy
        .revocation_list_path
        .unwrap_or_else(|| "demo/config/revoked.json".to_string());

    let bind_addr = format!("{}:{}", args.bind, args.port);
    let socket = UdpSocket::bind(&bind_addr).expect("bind failed");

    let start_log = json!({
        "ts": unix_ts(),
        "event": "executor_started",
        "bind": args.bind,
        "port": args.port,
        "policy": args.policy,
        "keys": args.keys,
        "allow_mock_signatures": args.allow_mock_signatures
    });
    println!("{}", start_log);

    let mut buf = [0u8; 65535];
    loop {
        let (len, addr) = match socket.recv_from(&mut buf) {
            Ok(res) => res,
            Err(_) => continue,
        };

        let parsed: Result<TaskRequest, _> = serde_json::from_slice(&buf[..len]);
        let request = match parsed {
            Ok(req) => req,
            Err(_) => {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "invalid_json",
                    "source": addr.to_string()
                });
                println!("{}", log);
                continue;
            }
        };

        if request.msg_type != "task_request" {
            let log = json!({
                "ts": unix_ts(),
                "event": "unexpected_message",
                "source": addr.to_string(),
                "message_type": request.msg_type
            });
            println!("{}", log);
            continue;
        }

        let mut details: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        if args.allow_mock_signatures {
            notes.push("signature verification skipped (mock mode)".to_string());
        }

        let now = unix_ts();
        if request.token.expires_at < now {
            details.push("token expired".to_string());
        }
        if request.token.audience != node_id {
            details.push("token audience mismatch".to_string());
        }
        if request.token.capability != request.requested_capability {
            details.push("capability mismatch".to_string());
        }

        if request.token.subject != request.commander_pubkey
            || keys
                .commander_pubkey
                .as_ref()
                .map(|k| k != &request.commander_pubkey)
                .unwrap_or(false)
        {
            details.push("token subject does not match commander_pubkey".to_string());
        }

        let revoked = load_string_list(&revoked_path);
        if revoked.iter().any(|t| t == &request.token.token_id) {
            details.push("token revoked".to_string());
        }

        if details.is_empty() {
            match replay_check_and_add(&replay_cache_path, &request.token.token_id) {
                Some(true) => {}
                Some(false) => {
                    details.push("replay detected (token_id already used)".to_string());
                }
                None => {
                    details.push("replay cache unavailable".to_string());
                }
            }
        }

        if !details.is_empty() {
            let reject = TaskRejected {
                version: 1,
                msg_type: "task_rejected".to_string(),
                task_id: request.task_id.clone(),
                details: details.clone(),
                notes: notes.clone(),
            };
            if let Ok(payload) = serde_json::to_vec(&reject) {
                let _ = socket.send_to(&payload, addr);
            }
            let log = json!({
                "ts": unix_ts(),
                "event": "task_rejected",
                "task_id": request.task_id,
                "details": details,
                "notes": notes
            });
            println!("{}", log);
            continue;
        }

        let payment_hash = sha256_hex(&[&request.task_id, &request.token.token_id, "payment"]);
        let accepted = TaskAccepted {
            version: 1,
            msg_type: "task_accepted".to_string(),
            task_id: request.task_id.clone(),
            payment_hash: payment_hash.clone(),
        };
        if let Ok(payload) = serde_json::to_vec(&accepted) {
            let _ = socket.send_to(&payload, addr);
        }
        let log = json!({
            "ts": unix_ts(),
            "event": "task_accepted",
            "task_id": request.task_id,
            "payment_hash": payment_hash
        });
        println!("{}", log);

        let proof_hash = sha256_hex(&[&accepted.task_id, &accepted.payment_hash, "proof"]);
        let proof = Proof {
            version: 1,
            msg_type: "proof".to_string(),
            task_id: accepted.task_id.clone(),
            proof_hash: proof_hash.clone(),
        };
        if let Ok(payload) = serde_json::to_vec(&proof) {
            let _ = socket.send_to(&payload, addr);
        }
        let log = json!({
            "ts": unix_ts(),
            "event": "proof_sent",
            "task_id": accepted.task_id,
            "proof_hash": proof_hash
        });
        println!("{}", log);
    }
}
