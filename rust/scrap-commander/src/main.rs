use clap::{Parser, ValueEnum};
use scrap_protocol::{
    bytes_to_hex, derive_payment_hash, hex_to_bytes, keypair_from_secret,
    pubkey_from_secret, sign_message_hash, MessageCodec, SpecMessage, SpecMessageCodec,
    SpecPaymentClaim, SpecPaymentLock, SpecTaskAccept, SpecTaskRequest, SpecTokenCodec,
    SpecVerifier, TokenCodec, Verifier,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::net::UdpSocket;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(name = "scrap-commander", about = "SCRAP commander (Rust demo)")]
struct Args {
    #[arg(long)]
    target_host: String,

    #[arg(long, default_value_t = 7227)]
    target_port: u16,

    #[arg(long)]
    token: Option<String>,

    #[arg(long)]
    keys: Option<String>,

    #[arg(long)]
    task_id: Option<String>,

    #[arg(long)]
    requested_capability: Option<String>,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    allow_mock_signatures: bool,

    #[arg(long, default_value_t = 15)]
    timeout: u64,

    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    #[arg(long, default_value_t = 0)]
    bind_port: u16,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    ping: bool,

    #[arg(long, value_enum, default_value = "demo")]
    mode: Mode,

    #[arg(long, default_value = "cmd:imaging:msi")]
    task_type: String,

    #[arg(long, default_value = "{}")]
    target_json: String,

    #[arg(long, default_value = "{}")]
    parameters_json: String,

    #[arg(long, default_value = "{}")]
    constraints_json: String,

    #[arg(long, default_value_t = 25000)]
    max_amount_sats: u64,

    #[arg(long, default_value_t = 144)]
    timeout_blocks: u32,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Mode {
    Demo,
    Spec,
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PaymentTerms {
    max_amount_sats: u64,
    timeout_blocks: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PaymentClaim {
    #[serde(rename = "type")]
    msg_type: String,
    task_id: String,
    correlation_id: String,
    payment_hash: String,
    preimage: String,
    timestamp: u64,
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result[..]);
    out
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn derive_demo_correlation_id(task_id: &str, token_id: &str) -> String {
    let seed = format!("{}:{}", task_id, token_id);
    to_hex(&sha256_bytes(seed.as_bytes()))
}

fn derive_demo_preimage(correlation_id: &str) -> [u8; 32] {
    sha256_bytes(correlation_id.as_bytes())
}

fn main() {
    let args = Args::parse();

    let bind_addr = format!("{}:{}", args.bind, args.bind_port);
    // Use a single socket for send + receive so replies return to the same source port.
    let socket = UdpSocket::bind(&bind_addr).expect("bind failed");
    let local_addr = socket.local_addr().expect("local addr failed");
    let target = format!("{}:{}", args.target_host, args.target_port);
    let bind_log = json!({
        "ts": unix_ts(),
        "event": "commander_socket_bound",
        "bind": bind_addr,
        "local_addr": local_addr.to_string()
    });
    println!("{}", bind_log);

    if args.ping {
        let ping_payload = format!("scrap_ping:{}", unix_ts());
        let send_result = socket.send_to(ping_payload.as_bytes(), &target);
        let log = match send_result {
            Ok(sent) => json!({
                "ts": unix_ts(),
                "event": "ping_sent",
                "target": target,
                "bytes": sent
            }),
            Err(err) => json!({
                "ts": unix_ts(),
                "event": "ping_send_failed",
                "target": target,
                "error": err.to_string()
            }),
        };
        println!("{}", log);

        let deadline = SystemTime::now() + Duration::from_secs(args.timeout);
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout set failed");

        let mut buf = [0u8; 65535];
        loop {
            if SystemTime::now() > deadline {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "ping_timeout"
                });
                println!("{}", log);
                return;
            }

            let (len, addr) = match socket.recv_from(&mut buf) {
                Ok(res) => res,
                Err(_) => continue,
            };

            let preview_len = len.min(32);
            let preview_hex = buf[..preview_len]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            let recv_log = json!({
                "ts": unix_ts(),
                "event": "udp_datagram_received",
                "source": addr.to_string(),
                "bytes": len,
                "hex_prefix": preview_hex
            });
            println!("{}", recv_log);

            if &buf[..len] == ping_payload.as_bytes() {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "ping_reply",
                    "source": addr.to_string()
                });
                println!("{}", log);
                return;
            }
        }
    }
    match args.mode {
        Mode::Demo => {
            let mut missing: Vec<&str> = Vec::new();
            if args.token.is_none() {
                missing.push("token");
            }
            if args.keys.is_none() {
                missing.push("keys");
            }
            if args.task_id.is_none() {
                missing.push("task_id");
            }
            if args.requested_capability.is_none() {
                missing.push("requested_capability");
            }
            if !missing.is_empty() {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "missing_required_args",
                    "missing": missing
                });
                println!("{}", log);
                return;
            }

            let token_path = args.token.as_ref().expect("token required");
            let token_raw = fs::read_to_string(token_path).expect("token read failed");
            let token: Token = serde_json::from_str(&token_raw).expect("token parse failed");

            let keys_path = args.keys.as_ref().expect("keys required");
            let keys_raw = fs::read_to_string(keys_path).expect("keys read failed");
            let keys_val: serde_json::Value =
                serde_json::from_str(&keys_raw).expect("keys parse failed");
            let commander_pubkey = keys_val
                .get("commander_pubkey")
                .and_then(|v| v.as_str())
                .expect("keys missing commander_pubkey")
                .to_string();

            let task_id = args.task_id.clone().expect("task_id required");
            let correlation_id = derive_demo_correlation_id(&task_id, &token.token_id);
            let payment_terms = PaymentTerms {
                max_amount_sats: args.max_amount_sats,
                timeout_blocks: args.timeout_blocks,
            };
            let preimage = derive_demo_preimage(&correlation_id);
            let payment_hash = to_hex(&sha256_bytes(&preimage));

            if args.allow_mock_signatures {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "commander_signature_mocked"
                });
                println!("{}", log);
            }

            let request = TaskRequest {
                version: 1,
                msg_type: "task_request".to_string(),
                task_id: task_id.clone(),
                requested_capability: args
                    .requested_capability
                    .clone()
                    .expect("requested_capability required"),
                payment_terms: payment_terms.clone(),
                correlation_id: correlation_id.clone(),
                token,
                commander_pubkey,
                commander_signature: if args.allow_mock_signatures {
                    "mock".to_string()
                } else {
                    "".to_string()
                },
            };

            let payload = serde_json::to_vec(&request).expect("serialize request failed");
            let send_result = socket.send_to(&payload, &target);
            let send_log = match send_result {
                Ok(sent) => json!({
                    "ts": unix_ts(),
                    "event": "task_request_sent",
                    "task_id": request.task_id,
                    "target": target,
                    "bytes": sent
                }),
                Err(err) => json!({
                    "ts": unix_ts(),
                    "event": "task_request_send_failed",
                    "task_id": request.task_id,
                    "target": target,
                    "error": err.to_string()
                }),
            };
            println!("{}", send_log);

            let lock = PaymentLock {
                msg_type: "payment_lock".to_string(),
                task_id: task_id.clone(),
                correlation_id: correlation_id.clone(),
                payment_hash: payment_hash.clone(),
                amount_sats: payment_terms.max_amount_sats,
                timeout_blocks: payment_terms.timeout_blocks,
                timestamp: unix_ts(),
            };
            let lock_payload = serde_json::to_vec(&lock).expect("serialize lock failed");
            let send_result = socket.send_to(&lock_payload, &target);
            let lock_log = match send_result {
                Ok(sent) => json!({
                    "ts": unix_ts(),
                    "event": "payment_lock_sent",
                    "bytes": sent
                }),
                Err(err) => json!({
                    "ts": unix_ts(),
                    "event": "payment_lock_send_failed",
                    "error": err.to_string()
                }),
            };
            println!("{}", lock_log);

            let deadline = SystemTime::now() + Duration::from_secs(args.timeout);
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("timeout set failed");

            let mut buf = [0u8; 65535];
            let mut proof_received = false;
            loop {
                if SystemTime::now() > deadline {
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "timeout_waiting_for_response"
                    });
                    println!("{}", log);
                    return;
                }

                let (len, addr) = match socket.recv_from(&mut buf) {
                    Ok(res) => res,
                    Err(_) => continue,
                };

                let preview_len = len.min(32);
                let preview_hex = buf[..preview_len]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                let recv_log = json!({
                    "ts": unix_ts(),
                    "event": "udp_datagram_received",
                    "source": addr.to_string(),
                    "bytes": len,
                    "hex_prefix": preview_hex
                });
                println!("{}", recv_log);

                let value: serde_json::Value = match serde_json::from_slice(&buf[..len]) {
                    Ok(v) => v,
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

                let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match msg_type {
                    "task_accepted" => {
                        let payment_hash =
                            value.get("payment_hash").and_then(|v| v.as_str()).unwrap_or("");
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "task_accepted",
                            "payment_hash": payment_hash
                        });
                        println!("{}", log);
                    }
                    "proof" => {
                        let proof_hash =
                            value.get("proof_hash").and_then(|v| v.as_str()).unwrap_or("");
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "proof_received",
                            "proof_hash": proof_hash
                        });
                        println!("{}", log);
                        proof_received = true;
                    }
                    "payment_claim" => {
                        if !proof_received {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_before_proof"
                            });
                            println!("{}", log);
                            continue;
                        }
                        let claim: PaymentClaim = match serde_json::from_value(value) {
                            Ok(claim) => claim,
                            Err(_) => continue,
                        };
                        if claim.correlation_id != correlation_id {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_correlation_mismatch"
                            });
                            println!("{}", log);
                            continue;
                        }
                        let derived_hash = to_hex(&sha256_bytes(&hex_to_bytes(&claim.preimage).unwrap_or_default()));
                        if derived_hash != claim.payment_hash || claim.payment_hash != payment_hash {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_hash_mismatch"
                            });
                            println!("{}", log);
                            continue;
                        }
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "payment_claim_received",
                            "payment_hash": claim.payment_hash
                        });
                        println!("{}", log);
                        return;
                    }
                    "task_rejected" => {
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "task_rejected",
                            "details": value.get("details").cloned().unwrap_or(json!([])),
                            "notes": value.get("notes").cloned().unwrap_or(json!([]))
                        });
                        println!("{}", log);
                        return;
                    }
                    _ => {}
                }
            }
        }
        Mode::Spec => {
            if args.allow_mock_signatures {
                panic!("spec mode does not allow mock signatures");
            }
            let mut missing: Vec<&str> = Vec::new();
            if args.token.is_none() {
                missing.push("token");
            }
            if args.keys.is_none() {
                missing.push("keys");
            }
            if args.task_id.is_none() {
                missing.push("task_id");
            }
            if !missing.is_empty() {
                let log = json!({
                    "ts": unix_ts(),
                    "event": "missing_required_args",
                    "missing": missing
                });
                println!("{}", log);
                return;
            }

            let token_path = args.token.as_ref().expect("token required");
            let token_bytes = fs::read(token_path).expect("token read failed");
            let token_codec = SpecTokenCodec;
            let _token = token_codec
                .decode_token(&token_bytes)
                .expect("spec token decode failed");

            let keys_path = args.keys.as_ref().expect("keys required");
            let keys_raw = fs::read_to_string(keys_path).expect("keys read failed");
            let keys_val: serde_json::Value =
                serde_json::from_str(&keys_raw).expect("keys parse failed");
            let commander_privkey = keys_val
                .get("commander_privkey")
                .and_then(|v| v.as_str())
                .expect("keys missing commander_privkey");
            let commander_key = keypair_from_secret(commander_privkey)
                .expect("commander keypair failed");
            let executor_pubkey = if let Some(privkey) = keys_val
                .get("executor_privkey")
                .and_then(|v| v.as_str())
            {
                pubkey_from_secret(privkey).expect("executor pubkey derive failed")
            } else if let Some(pubkey) = keys_val.get("executor_pubkey").and_then(|v| v.as_str()) {
                hex_to_bytes(pubkey).expect("executor pubkey hex parse failed")
            } else {
                panic!("keys missing executor pubkey/privkey");
            };

            let mut request = SpecTaskRequest {
                task_id: args.task_id.clone().expect("task_id required"),
                timestamp: unix_ts() as u32,
                capability_token: token_bytes.clone(),
                delegation_chain: Vec::new(),
                task_type: args.task_type.clone(),
                target_json: args.target_json.clone(),
                parameters_json: args.parameters_json.clone(),
                constraints_json: args.constraints_json.clone(),
                payment_max_sats: args.max_amount_sats,
                timeout_blocks: args.timeout_blocks,
                commander_signature: [0u8; 64],
            };
            let signing_hash = request.commander_signing_hash();
            request.commander_signature =
                sign_message_hash(signing_hash, &commander_key).expect("sign request failed");

            let codec = SpecMessageCodec;
            let payload = codec
                .encode_message(&SpecMessage::TaskRequest(request.clone()))
                .expect("spec request encode failed");
            let send_result = socket.send_to(&payload, &target);
            let send_log = match send_result {
                Ok(sent) => json!({
                    "ts": unix_ts(),
                    "event": "task_request_sent",
                    "mode": "spec",
                    "task_id": request.task_id,
                    "target": target,
                    "bytes": sent
                }),
                Err(err) => json!({
                    "ts": unix_ts(),
                    "event": "task_request_send_failed",
                    "mode": "spec",
                    "task_id": request.task_id,
                    "target": target,
                    "error": err.to_string()
                }),
            };
            println!("{}", send_log);

            let correlation_id = request.request_hash();
            let expected_payment_hash = derive_payment_hash(correlation_id);
            let lock = SpecPaymentLock {
                task_id: request.task_id.clone(),
                correlation_id,
                payment_hash: expected_payment_hash,
                amount_sats: request.payment_max_sats,
                timeout_blocks: request.timeout_blocks,
                timestamp: unix_ts() as u32,
            };
            let lock_payload = codec
                .encode_message(&SpecMessage::PaymentLock(lock.clone()))
                .expect("encode lock failed");
            let send_result = socket.send_to(&lock_payload, &target);
            let lock_log = match send_result {
                Ok(sent) => json!({
                    "ts": unix_ts(),
                    "event": "payment_lock_sent",
                    "mode": "spec",
                    "bytes": sent
                }),
                Err(err) => json!({
                    "ts": unix_ts(),
                    "event": "payment_lock_send_failed",
                    "mode": "spec",
                    "error": err.to_string()
                }),
            };
            println!("{}", lock_log);

            let verifier = SpecVerifier {
                operator_pubkey: Vec::new(),
                executor_pubkey: executor_pubkey.clone(),
            };

            let deadline = SystemTime::now() + Duration::from_secs(args.timeout);
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("timeout set failed");

            let mut buf = [0u8; 65535];
            let mut accepted: Option<SpecTaskAccept> = None;
            let mut proof_received = false;
            loop {
                if SystemTime::now() > deadline {
                    let log = json!({
                        "ts": unix_ts(),
                        "event": "timeout_waiting_for_response"
                    });
                    println!("{}", log);
                    return;
                }

                let (len, addr) = match socket.recv_from(&mut buf) {
                    Ok(res) => res,
                    Err(_) => continue,
                };

                let preview_len = len.min(32);
                let preview_hex = buf[..preview_len]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                let recv_log = json!({
                    "ts": unix_ts(),
                    "event": "udp_datagram_received",
                    "source": addr.to_string(),
                    "bytes": len,
                    "hex_prefix": preview_hex
                });
                println!("{}", recv_log);

                let message = match codec.decode_message(&buf[..len]) {
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
                    SpecMessage::TaskAccept(accept) => {
                        let request_hash = request.request_hash();
                        if let Err(err) = verifier.verify_accept(&accept, request_hash) {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "task_accept_invalid",
                                "error": err.to_string()
                            });
                            println!("{}", log);
                            continue;
                        }
                        if accept.amount_sats > request.payment_max_sats {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "task_accept_amount_exceeds_offer",
                                "amount_sats": accept.amount_sats,
                                "max_amount_sats": request.payment_max_sats
                            });
                            println!("{}", log);
                            continue;
                        }
                        if accept.payment_hash != expected_payment_hash {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "task_accept_payment_hash_mismatch"
                            });
                            println!("{}", log);
                            continue;
                        }
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "task_accepted",
                            "mode": "spec",
                            "payment_hash": bytes_to_hex(&accept.payment_hash)
                        });
                        println!("{}", log);
                        accepted = Some(accept);
                    }
                    SpecMessage::ProofOfExecution(proof) => {
                        if let Some(accept) = &accepted {
                            if proof.payment_hash != accept.payment_hash {
                                let log = json!({
                                    "ts": unix_ts(),
                                    "event": "proof_payment_hash_mismatch"
                                });
                                println!("{}", log);
                                continue;
                            }
                        }
                        if let Err(err) = verifier.verify_proof(&proof) {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "proof_invalid",
                                "error": err.to_string()
                            });
                            println!("{}", log);
                            continue;
                        }
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "proof_received",
                            "mode": "spec",
                            "output_hash": bytes_to_hex(&proof.output_hash),
                            "token_id": bytes_to_hex(&proof.task_token_id)
                        });
                        println!("{}", log);
                        proof_received = true;
                    }
                    SpecMessage::PaymentClaim(claim) => {
                        if !proof_received {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_before_proof"
                            });
                            println!("{}", log);
                            continue;
                        }
                        if claim.correlation_id != correlation_id {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_correlation_mismatch"
                            });
                            println!("{}", log);
                            continue;
                        }
                        let derived_hash = scrap_protocol::sha256(&claim.preimage);
                        if derived_hash != claim.payment_hash || claim.payment_hash != expected_payment_hash {
                            let log = json!({
                                "ts": unix_ts(),
                                "event": "payment_claim_hash_mismatch"
                            });
                            println!("{}", log);
                            continue;
                        }
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "payment_claim_received",
                            "mode": "spec",
                            "payment_hash": bytes_to_hex(&claim.payment_hash)
                        });
                        println!("{}", log);
                        return;
                    }
                    SpecMessage::TaskReject(reject) => {
                        let log = json!({
                            "ts": unix_ts(),
                            "event": "task_rejected",
                            "mode": "spec",
                            "reason": reject.reason,
                            "details": reject.details
                        });
                        println!("{}", log);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}
