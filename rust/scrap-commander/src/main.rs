use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    token: String,

    #[arg(long)]
    keys: String,

    #[arg(long)]
    task_id: String,

    #[arg(long)]
    requested_capability: String,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    allow_mock_signatures: bool,

    #[arg(long, default_value_t = 15)]
    timeout: u64,
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
    token: Token,
    commander_pubkey: String,
    commander_signature: String,
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn main() {
    let args = Args::parse();

    let token_raw = fs::read_to_string(&args.token).expect("token read failed");
    let token: Token = serde_json::from_str(&token_raw).expect("token parse failed");

    let keys_raw = fs::read_to_string(&args.keys).expect("keys read failed");
    let keys_val: serde_json::Value = serde_json::from_str(&keys_raw).expect("keys parse failed");
    let commander_pubkey = keys_val
        .get("commander_pubkey")
        .and_then(|v| v.as_str())
        .expect("keys missing commander_pubkey")
        .to_string();

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
        task_id: args.task_id.clone(),
        requested_capability: args.requested_capability.clone(),
        token,
        commander_pubkey,
        commander_signature: if args.allow_mock_signatures { "mock".to_string() } else { "".to_string() },
    };

    let payload = serde_json::to_vec(&request).expect("serialize request failed");
    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind failed");
    let target = format!("{}:{}", args.target_host, args.target_port);
    let _ = socket.send_to(&payload, &target);

    let log = json!({
        "ts": unix_ts(),
        "event": "task_request_sent",
        "task_id": args.task_id,
        "target": target
    });
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
                "event": "timeout_waiting_for_response"
            });
            println!("{}", log);
            return;
        }

        let (len, _) = match socket.recv_from(&mut buf) {
            Ok(res) => res,
            Err(_) => continue,
        };

        let value: serde_json::Value = match serde_json::from_slice(&buf[..len]) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match msg_type {
            "task_accepted" => {
                let payment_hash = value.get("payment_hash").and_then(|v| v.as_str()).unwrap_or("");
                let log = json!({
                    "ts": unix_ts(),
                    "event": "task_accepted",
                    "payment_hash": payment_hash
                });
                println!("{}", log);
            }
            "proof" => {
                let proof_hash = value.get("proof_hash").and_then(|v| v.as_str()).unwrap_or("");
                let log = json!({
                    "ts": unix_ts(),
                    "event": "proof_received",
                    "proof_hash": proof_hash
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
