use clap::{Parser, ValueEnum};
use scrap_protocol::{
    bytes_to_hex, derive_payment_hash, derive_preimage, hex_to_bytes, keypair_from_secret,
    normalize_pubkey, pubkey_from_secret, sign_message_hash, MessageCodec, SettlementState,
    SpecMessage, SpecMessageCodec, SpecPaymentClaim, SpecProofOfExecution, SpecTaskAccept,
    SpecTaskReject, SpecTokenCodec, SpecVerifier, TokenCodec, Verifier,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::UdpSocket;
use std::path::Path;
use std::collections::HashMap;
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

    #[arg(long, action = clap::ArgAction::SetTrue)]
    debug_echo: bool,

    #[arg(long, value_enum, default_value = "demo")]
    mode: Mode,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Mode {
    Demo,
    Spec,
}

#[derive(Debug, Deserialize)]
struct Policy {
    node_id: Option<String>,
    executor_pubkey: Option<String>,
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
    payment_terms: PaymentTerms,
    correlation_id: String,
    token: Token,
    commander_pubkey: String,
    commander_signature: String,
}

#[derive(Debug, Deserialize)]
struct PaymentTerms {
    max_amount_sats: u64,
    timeout_blocks: u32,
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

#[derive(Debug, Deserialize)]
struct PaymentLock {
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    correlation_id: String,
    payment_hash: String,
    amount_sats: u64,
    timeout_blocks: u32,
    timestamp: u64,
}

#[derive(Debug, Serialize)]
struct PaymentClaim {
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    correlation_id: String,
    payment_hash: String,
    preimage: String,
    timestamp: u64,
}

struct SpecTaskState {
    request: SpecTaskRequest,
    token_id: [u8; 16],
    settlement: scrap_protocol::SettlementState,
}

struct DemoTaskState {
    request: TaskRequest,
    token_id: String,
    correlation_id: String,
    payment_hash: String,
    locked: bool,
}

struct SpecKeys {
    operator_pubkey: Vec<u8>,
    executor_pubkey: Vec<u8>,
    executor_privkey: String,
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

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result[..]);
    out
}

fn derive_demo_correlation_id(task_id: &str, token_id: &str) -> String {
    let seed = format!("{}:{}", task_id, token_id);
    to_hex(&sha256_bytes(seed.as_bytes()))
}

fn derive_demo_preimage(correlation_id: &str) -> [u8; 32] {
    sha256_bytes(correlation_id.as_bytes())
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
    if matches!(args.mode, Mode::Spec) && args.allow_mock_signatures {
        panic!("spec mode does not allow mock signatures");
    }

    let policy: Policy = read_json_file(&args.policy).unwrap_or(Policy {
        node_id: None,
        executor_pubkey: None,
        replay_cache_path: None,
        revocation_list_path: None,
    });
    let keys: Keys = read_json_file(&args.keys).unwrap_or(Keys {
        commander_pubkey: None,
    });
    let spec_keys = if matches!(args.mode, Mode::Spec) {
        let keys_raw = fs::read_to_string(&args.keys).expect("keys read failed");
        let keys_val: serde_json::Value =
            serde_json::from_str(&keys_raw).expect("keys parse failed");
        let operator_pubkey = if let Some(privkey) = keys_val
            .get("operator_privkey")
            .and_then(|v| v.as_str())
        {
            pubkey_from_secret(privkey).expect("operator pubkey derive failed")
        } else if let Some(pubkey) = keys_val.get("operator_pubkey").and_then(|v| v.as_str()) {
            hex_to_bytes(pubkey).expect("operator pubkey hex parse failed")
        } else {
            panic!("keys missing operator_pubkey/operator_privkey");
        };
        let executor_privkey = keys_val
            .get("executor_privkey")
            .and_then(|v| v.as_str())
            .expect("keys missing executor_privkey");
        let executor_pubkey =
            pubkey_from_secret(executor_privkey).expect("executor pubkey derive failed");
        Some(SpecKeys {
            operator_pubkey,
            executor_pubkey,
            executor_privkey: executor_privkey.to_string(),
        })
    } else {
        None
    };

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
    let mode_label = match args.mode {
        Mode::Demo => "demo",
        Mode::Spec => "spec",
    };

    let start_log = json!({
        "ts": unix_ts(),
        "event": "executor_started",
        "bind": args.bind,
        "port": args.port,
        "policy": args.policy,
        "keys": args.keys,
        "allow_mock_signatures": args.allow_mock_signatures,
        "debug_echo": args.debug_echo,
        "mode": mode_label
    });
    println!("{}", start_log);

    let mut demo_states: HashMap<String, DemoTaskState> = HashMap::new();
    let mut spec_states: HashMap<String, SpecTaskState> = HashMap::new();
    let spec_codec = SpecMessageCodec;
    let token_codec = SpecTokenCodec;
    let spec_verifier = spec_keys.as_ref().map(|keys| {
        let executor_pubkey = if let Some(config_hex) = &policy.executor_pubkey {
            let config_bytes =
                hex_to_bytes(config_hex).expect("policy executor_pubkey hex parse failed");
            let config_norm =
                normalize_pubkey(&config_bytes).expect("policy executor_pubkey invalid");
            let key_norm =
                normalize_pubkey(&keys.executor_pubkey).expect("executor pubkey invalid");
            if config_norm != key_norm {
                panic!("policy executor_pubkey does not match executor_privkey");
            }
            config_bytes
        } else {
            keys.executor_pubkey.clone()
        };
        SpecVerifier {
            operator_pubkey: keys.operator_pubkey.clone(),
            executor_pubkey,
        }
    });

    let mut buf = [0u8; 65535];
    loop {
        let (len, addr) = match socket.recv_from(&mut buf) {
            Ok(res) => res,
            Err(err) => {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "udp_recv_error",
                    "error": err.to_string()
                });
                println!("{}", log);
                continue;
            }
        };

        let preview_len = len.min(32);
        let preview_hex = to_hex(&buf[..preview_len]);
        let recv_log = json!({
            "ts": unix_ts(),
            "event": "udp_datagram_received",
            "source": addr.to_string(),
            "bytes": len,
            "hex_prefix": preview_hex
        });
        println!("{}", recv_log);

        if args.debug_echo {
            let send_result = socket.send_to(&buf[..len], addr);
            let log = match send_result {
                Ok(sent) => json!({
                    "ts": unix_ts(),
                    "event": "debug_echo_sent",
                    "destination": addr.to_string(),
                    "bytes": sent
                }),
                Err(err) => json!({
                    "ts": unix_ts(),
                    "event": "debug_echo_failed",
                    "destination": addr.to_string(),
                    "error": err.to_string()
                }),
            };
            println!("{}", log);
            continue;
        }

        if matches!(args.mode, Mode::Demo) {
            let value: serde_json::Value = match serde_json::from_slice(&buf[..len]) {
                Ok(value) => value,
                Err(_) => {
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "invalid_json",
                        "source": addr.to_string(),
                        "bytes": len
                    });
                    println!("{}", log);
                    continue;
                }
            };

            let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match msg_type {
                "task_request" => {
                    let request: TaskRequest = match serde_json::from_value(value) {
                        Ok(req) => req,
                        Err(_) => {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "invalid_task_request",
                                "source": addr.to_string()
                            });
                            println!("{}", log);
                            continue;
                        }
                    };

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

                    let expected_correlation =
                        derive_demo_correlation_id(&request.task_id, &request.token.token_id);
                    if request.correlation_id != expected_correlation {
                        details.push("correlation_id mismatch".to_string());
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
                            let send_result = socket.send_to(&payload, addr);
                            let log = match send_result {
                                Ok(sent) => json!({
                                    "ts": unix_ts(),
                                    "event": "task_rejected_sent",
                                    "destination": addr.to_string(),
                                    "bytes": sent
                                }),
                                Err(err) => json!({
                                    "ts": unix_ts(),
                                    "event": "task_rejected_send_failed",
                                    "destination": addr.to_string(),
                                    "error": err.to_string()
                                }),
                            };
                            println!("{}", log);
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

                    let preimage = derive_demo_preimage(&request.correlation_id);
                    let payment_hash = to_hex(&sha256_bytes(&preimage));
                    if demo_states.contains_key(&request.task_id) {
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "task_request_duplicate",
                            "task_id": request.task_id
                        });
                        println!("{}", log);
                        continue;
                    }
                    demo_states.insert(
                        request.task_id.clone(),
                        DemoTaskState {
                            request: request.clone(),
                            token_id: request.token.token_id.clone(),
                            correlation_id: request.correlation_id.clone(),
                            payment_hash: payment_hash.clone(),
                            locked: false,
                        },
                    );

                    let log = json!({
                        "ts": unix_ts(),
                        "event": "task_request_received",
                        "task_id": request.task_id,
                        "correlation_id": request.correlation_id,
                        "payment_hash": payment_hash
                    });
                    println!("{}", log);
                }
                "payment_lock" => {
                    let lock: PaymentLock = match serde_json::from_value(value) {
                        Ok(lock) => lock,
                        Err(_) => {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "invalid_payment_lock",
                                "source": addr.to_string()
                            });
                            println!("{}", log);
                            continue;
                        }
                    };

                    let state = match demo_states.get_mut(&lock.task_id) {
                        Some(state) => state,
                        None => {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_lock_unknown_task",
                                "task_id": lock.task_id
                            });
                            println!("{}", log);
                            continue;
                        }
                    };

                    let mut details: Vec<String> = Vec::new();
                    if lock.correlation_id != state.correlation_id {
                        details.push("lock correlation_id mismatch".to_string());
                    }
                    if lock.payment_hash != state.payment_hash {
                        details.push("lock payment_hash mismatch".to_string());
                    }
                    if lock.amount_sats > state.request.payment_terms.max_amount_sats {
                        details.push("lock amount exceeds offer".to_string());
                    }
                    if lock.timeout_blocks != state.request.payment_terms.timeout_blocks {
                        details.push("lock timeout mismatch".to_string());
                    }
                    let now = unix_ts();
                    if now > lock.timestamp + lock.timeout_blocks as u64 {
                        details.push("lock timeout elapsed".to_string());
                    }

                    if !details.is_empty() {
                        let reject = TaskRejected {
                            version: 1,
                            msg_type: "task_rejected".to_string(),
                            task_id: lock.task_id.clone(),
                            details: details.clone(),
                            notes: Vec::new(),
                        };
                        if let Ok(payload) = serde_json::to_vec(&reject) {
                            let send_result = socket.send_to(&payload, addr);
                            let log = match send_result {
                                Ok(sent) => json!({
                                    "ts": unix_ts(),
                                    "event": "task_rejected_sent",
                                    "destination": addr.to_string(),
                                    "bytes": sent
                                }),
                                Err(err) => json!({
                                    "ts": unix_ts(),
                                    "event": "task_rejected_send_failed",
                                    "destination": addr.to_string(),
                                    "error": err.to_string()
                                }),
                            };
                            println!("{}", log);
                        }
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "payment_lock_rejected",
                            "task_id": lock.task_id,
                            "details": details
                        });
                        println!("{}", log);
                        continue;
                    }

                    state.locked = true;
                    let accepted = TaskAccepted {
                        version: 1,
                        msg_type: "task_accepted".to_string(),
                        task_id: lock.task_id.clone(),
                        payment_hash: state.payment_hash.clone(),
                    };
                    if let Ok(payload) = serde_json::to_vec(&accepted) {
                        let send_result = socket.send_to(&payload, addr);
                        let log = match send_result {
                            Ok(sent) => json!({
                                "ts": unix_ts(),
                                "event": "task_accepted_sent",
                                "destination": addr.to_string(),
                                "bytes": sent
                            }),
                            Err(err) => json!({
                                "ts": unix_ts(),
                                "event": "task_accepted_send_failed",
                                "destination": addr.to_string(),
                                "error": err.to_string()
                            }),
                        };
                        println!("{}", log);
                    }
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "task_accepted",
                        "task_id": lock.task_id,
                        "payment_hash": state.payment_hash.clone()
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
                        let send_result = socket.send_to(&payload, addr);
                        let log = match send_result {
                            Ok(sent) => json!({
                                "ts": unix_ts(),
                                "event": "proof_sent_udp",
                                "destination": addr.to_string(),
                                "bytes": sent
                            }),
                            Err(err) => json!({
                                "ts": unix_ts(),
                                "event": "proof_send_failed",
                                "destination": addr.to_string(),
                                "error": err.to_string()
                            }),
                        };
                        println!("{}", log);
                    }
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "proof_sent",
                        "task_id": accepted.task_id,
                        "proof_hash": proof_hash
                    });
                    println!("{}", log);

                    let preimage = derive_demo_preimage(&state.correlation_id);
                    let claim = PaymentClaim {
                        msg_type: "payment_claim".to_string(),
                        task_id: lock.task_id.clone(),
                        correlation_id: state.correlation_id.clone(),
                        payment_hash: state.payment_hash.clone(),
                        preimage: to_hex(&preimage),
                        timestamp: unix_ts(),
                    };
                    if let Ok(payload) = serde_json::to_vec(&claim) {
                        let send_result = socket.send_to(&payload, addr);
                        let log = match send_result {
                            Ok(sent) => json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_sent",
                                "destination": addr.to_string(),
                                "bytes": sent
                            }),
                            Err(err) => json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_send_failed",
                                "destination": addr.to_string(),
                                "error": err.to_string()
                            }),
                        };
                        println!("{}", log);
                    }
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "payment_claim_sent",
                        "task_id": lock.task_id,
                        "payment_hash": state.payment_hash.clone()
                    });
                    println!("{}", log);
                }
                _ => {
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "unexpected_message",
                        "source": addr.to_string(),
                        "message_type": msg_type
                    });
                    println!("{}", log);
                }
            }
            continue;
        }

        // Spec mode handling
        let spec_keys_ref = match &spec_keys {
            Some(keys) => keys,
            None => {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "spec_keys_missing"
                });
                println!("{}", log);
                continue;
            }
        };
        let message = match spec_codec.decode_message(&buf[..len]) {
            Ok(msg) => msg,
            Err(err) => {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "invalid_message",
                    "error": err.to_string(),
                    "source": addr.to_string()
                });
                println!("{}", log);
                continue;
            }
        };

        match message {
            SpecMessage::TaskRequest(request) => {
                let verifier = spec_verifier.as_ref().expect("spec verifier missing");
                if let Err(err) = verifier.verify_request(&request, unix_ts()) {
                    let reject = SpecTaskReject {
                        task_id: request.task_id.clone(),
                        reason: "verification_failed".to_string(),
                        details: err.to_string(),
                        timestamp: unix_ts() as u32,
                    };
                    if let Ok(payload) = spec_codec.encode_message(&SpecMessage::TaskReject(reject.clone())) {
                        let _ = socket.send_to(&payload, addr);
                    }
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "task_rejected",
                        "mode": "spec",
                        "task_id": request.task_id,
                        "details": err.to_string()
                    });
                    println!("{}", log);
                    continue;
                }

                let token = match token_codec.decode_token(&request.capability_token) {
                    Ok(token) => token,
                    Err(err) => {
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "token_decode_failed",
                            "error": err.to_string()
                        });
                        println!("{}", log);
                        continue;
                    }
                };

                let now = unix_ts() as u32;
                let token_id_hex = bytes_to_hex(&token.token_id);
                let revoked = load_string_list(&revoked_path);
                if revoked.iter().any(|t| t == &token_id_hex) {
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "token_revoked",
                        "mode": "spec",
                        "task_id": request.task_id
                    });
                    println!("{}", log);
                    continue;
                }
                match replay_check_and_add(&replay_cache_path, &token_id_hex) {
                    Some(true) => {}
                    Some(false) => {
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "replay_detected",
                            "mode": "spec",
                            "task_id": request.task_id
                        });
                        println!("{}", log);
                        continue;
                    }
                    None => {
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "replay_cache_unavailable",
                            "mode": "spec"
                        });
                        println!("{}", log);
                        continue;
                    }
                }
                let settlement = SettlementState::new(&request);
                spec_states.insert(
                    request.task_id.clone(),
                    SpecTaskState {
                        request: request.clone(),
                        token_id: token.token_id,
                        settlement,
                    },
                );

                let log = json!({
                    "ts": unix_ts(),
                    "event": "task_request_received",
                    "mode": "spec",
                    "task_id": request.task_id,
                    "correlation_id": bytes_to_hex(&request.request_hash())
                });
                println!("{}", log);
            }
            SpecMessage::PaymentLock(lock) => {
                if let Some(state) = spec_states.get_mut(&lock.task_id) {
                    let now = unix_ts();
                    if let Err(err) = state.settlement.validate_lock(&lock, now) {
                        let reject = SpecTaskReject {
                            task_id: lock.task_id.clone(),
                            reason: "lock_invalid".to_string(),
                            details: err.to_string(),
                            timestamp: unix_ts() as u32,
                        };
                        if let Ok(payload) =
                            spec_codec.encode_message(&SpecMessage::TaskReject(reject.clone()))
                        {
                            let _ = socket.send_to(&payload, addr);
                        }
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "payment_lock_rejected",
                            "mode": "spec",
                            "task_id": lock.task_id,
                            "details": err.to_string()
                        });
                        println!("{}", log);
                        state.settlement.phase = scrap_protocol::SettlementPhase::Rejected;
                        continue;
                    }

                    state.settlement.mark_locked();

                    let mut accept = SpecTaskAccept {
                        task_id: lock.task_id.clone(),
                        timestamp: now as u32,
                        in_reply_to: state.settlement.correlation_id,
                        payment_hash: lock.payment_hash,
                        amount_sats: lock.amount_sats,
                        expiry_sec: 3600,
                        description: lock.task_id.clone(),
                        estimated_duration_sec: 45,
                        earliest_start: now as u32 + 5,
                        data_volume_mb: 250,
                        quality_estimate: 920,
                        executor_signature: [0u8; 64],
                    };
                    let keypair = keypair_from_secret(&spec_keys_ref.executor_privkey)
                        .expect("executor keypair failed");
                    accept.executor_signature =
                        sign_message_hash(accept.executor_signing_hash(), &keypair)
                            .expect("sign accept failed");
                    if let Ok(payload) =
                        spec_codec.encode_message(&SpecMessage::TaskAccept(accept.clone()))
                    {
                        let _ = socket.send_to(&payload, addr);
                    }
                    state.settlement.phase = scrap_protocol::SettlementPhase::Accepted;
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "task_accepted",
                        "mode": "spec",
                        "task_id": accept.task_id,
                        "payment_hash": bytes_to_hex(&accept.payment_hash)
                    });
                    println!("{}", log);

                    let output_hash = scrap_protocol::sha256(
                        format!("{}:{}", lock.task_id, "output").as_bytes(),
                    );
                    let mut proof = SpecProofOfExecution {
                        task_id: lock.task_id.clone(),
                        task_token_id: state.token_id,
                        payment_hash: lock.payment_hash,
                        output_hash,
                        execution_timestamp: unix_ts() as u32,
                        executor_pubkey: spec_keys_ref.executor_pubkey.clone(),
                        executor_signature: [0u8; 64],
                    };
                    proof.executor_signature =
                        sign_message_hash(proof.proof_hash(), &keypair).expect("sign proof failed");
                    if let Ok(payload) =
                        spec_codec.encode_message(&SpecMessage::ProofOfExecution(proof.clone()))
                    {
                        let _ = socket.send_to(&payload, addr);
                    }
                    state.settlement.phase = scrap_protocol::SettlementPhase::ProofSent;
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "proof_sent",
                        "mode": "spec",
                        "task_id": lock.task_id,
                        "payment_hash": bytes_to_hex(&lock.payment_hash)
                    });
                    println!("{}", log);

                    let preimage = derive_preimage(state.settlement.correlation_id);
                    let claim = SpecPaymentClaim {
                        task_id: lock.task_id.clone(),
                        correlation_id: state.settlement.correlation_id,
                        payment_hash: derive_payment_hash(state.settlement.correlation_id),
                        preimage,
                        timestamp: unix_ts() as u32,
                    };
                    if let Ok(payload) =
                        spec_codec.encode_message(&SpecMessage::PaymentClaim(claim.clone()))
                    {
                        let _ = socket.send_to(&payload, addr);
                    }
                    state.settlement.phase = scrap_protocol::SettlementPhase::Claimed;
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "payment_claim_sent",
                        "mode": "spec",
                        "task_id": lock.task_id,
                        "payment_hash": bytes_to_hex(&lock.payment_hash)
                    });
                    println!("{}", log);
                } else {
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "payment_lock_unknown_task",
                        "task_id": lock.task_id
                    });
                    println!("{}", log);
                }
            }
            _ => {}
        }
    }
}
