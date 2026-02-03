use clap::Parser;
use scrap_core_lite::{
    build_task_request, decode_envelope, encode_envelope, Payload, TaskRequest, Token,
};
use scrap_linux_udp::{hex_encode, hex_preview, load_routes};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::net::UdpSocket;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(name = "scrap-orchestrator", about = "SCRAP orchestrator (Rust demo)")]
struct Args {
    #[arg(long, default_value = "ORCH")]
    node_id: String,

    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    #[arg(long, default_value_t = 7331)]
    port: u16,

    #[arg(long, default_value = "inventory/routes.json")]
    routes: String,

    #[arg(long, default_value = "BBB-01")]
    target: String,

    #[arg(long, default_value = "demo/config/keys.json")]
    keys: String,

    #[arg(long, default_value = "demo.hash")]
    command: String,

    #[arg(long, default_value = "123")]
    args: String,

    #[arg(long)]
    token_subject: Option<String>,

    #[arg(long)]
    token_audience: Option<String>,

    #[arg(long)]
    token_capability: Option<String>,

    #[arg(long, default_value_t = 10)]
    timeout: u64,
}

#[derive(Debug, Deserialize)]
struct KeysFile {
    commander_pubkey: String,
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sha256_bytes(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

fn main() {
    let args = Args::parse();

    let keys_raw = fs::read_to_string(&args.keys).expect("keys read failed");
    let keys: KeysFile = serde_json::from_str(&keys_raw).expect("keys parse failed");

    let routes = load_routes(&args.routes, &args.node_id).expect("routes load failed");
    let next_hop = routes
        .next_hop(&args.target)
        .expect("no route to target")
        .to_string();

    let trace_seed = format!("{}:{}:{}", args.node_id, args.target, unix_ts());
    let trace_id = sha256_bytes(trace_seed.as_bytes());
    let trace_id = trace_id[..scrap_core_lite::TRACE_ID_LEN].to_vec();

    let token_seed = format!("{}:{}:{}", keys.commander_pubkey, args.target, unix_ts());
    let token_id = sha256_bytes(token_seed.as_bytes());
    let token_id = token_id[..scrap_core_lite::TOKEN_ID_LEN].to_vec();

    let now = unix_ts();
    let token_subject = args
        .token_subject
        .clone()
        .unwrap_or_else(|| keys.commander_pubkey.clone());
    let token_audience = args.token_audience.clone().unwrap_or_else(|| args.target.clone());
    let token_capability = args
        .token_capability
        .clone()
        .unwrap_or_else(|| args.command.clone());

    let token = Token {
        token_id,
        subject: token_subject,
        audience: token_audience,
        capability: token_capability,
        issued_at: now,
        expires_at: now + 600,
    };

    let task = TaskRequest {
        token,
        command: args.command.clone(),
        args: args.args.clone(),
        reply_to: args.node_id.clone(),
        commander_pubkey: keys.commander_pubkey.clone(),
    };

    let env = build_task_request(
        trace_id.clone(),
        args.node_id.clone(),
        args.target.clone(),
        4,
        task,
    );

    let mut buf = Vec::new();
    encode_envelope(&env, &mut buf).expect("encode failed");

    println!("{}", json!({
        "ts": unix_ts(),
        "event": "task_request_encoded",
        "len": buf.len(),
        "preview": hex_preview(&buf, 32)
    }));

    let bind_addr = format!("{}:{}", args.bind, args.port);
    let socket = UdpSocket::bind(&bind_addr).expect("bind failed");
    let _ = socket.send_to(&buf, &next_hop);

    println!("{}", json!({
        "ts": unix_ts(),
        "event": "task_request_sent",
        "trace_id": hex_encode(&trace_id),
        "next_hop": next_hop
    }));

    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout set failed");
    let deadline = SystemTime::now() + Duration::from_secs(args.timeout);

    let mut recv_buf = [0u8; 2048];
    loop {
        if SystemTime::now() > deadline {
            eprintln!("timeout waiting for result");
            std::process::exit(2);
        }

        let (len, _) = match socket.recv_from(&mut recv_buf) {
            Ok(res) => res,
            Err(_) => continue,
        };

        let env = match decode_envelope(&recv_buf[..len]) {
            Ok(env) => env,
            Err(_) => continue,
        };

        if env.trace_id != trace_id {
            println!("{}", json!({
                "ts": unix_ts(),
                "event": "trace_id_mismatch",
                "expected": hex_encode(&trace_id),
                "got": hex_encode(&env.trace_id)
            }));
            std::process::exit(3);
        }

        match env.payload {
            Payload::TaskResult(result) => {
                println!("{}", json!({
                    "ts": unix_ts(),
                    "event": "task_result",
                    "trace_id": hex_encode(&trace_id),
                    "status": result.status,
                    "output_digest": hex_encode(&result.output_digest)
                }));
                std::process::exit(0);
            }
            Payload::TaskRejected(reject) => {
                println!("{}", json!({
                    "ts": unix_ts(),
                    "event": "task_rejected",
                    "trace_id": hex_encode(&trace_id),
                    "reason": reject.reason,
                    "details": reject.details
                }));
                std::process::exit(1);
            }
            _ => {}
        }
    }
}
