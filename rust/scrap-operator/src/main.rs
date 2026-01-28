use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(name = "scrap-operator", about = "SCRAP operator (Rust demo)")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(name = "issue-token")]
    IssueToken {
        #[arg(long)]
        keys: String,

        #[arg(long)]
        out: String,

        #[arg(long)]
        meta_out: Option<String>,

        #[arg(long)]
        subject: String,

        #[arg(long)]
        audience: String,

        #[arg(long)]
        capability: String,

        #[arg(long, default_value_t = 3600)]
        expires_in: u64,

        #[arg(long)]
        token_id: Option<String>,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        allow_mock_signature: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
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

fn ensure_parent(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}

fn write_json<T: Serialize>(path: &str, value: &T) {
    ensure_parent(path);
    let payload = serde_json::to_vec_pretty(value).expect("serialize failed");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("open failed");
    let _ = file.write_all(&payload);
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::IssueToken {
            keys: _keys,
            out,
            meta_out,
            subject,
            audience,
            capability,
            expires_in,
            token_id,
            allow_mock_signature,
        } => {
            let issued_at = unix_ts();
            let expires_at = issued_at + expires_in;
            let token_id = token_id.unwrap_or_else(|| {
                let seed = format!("{}:{}:{}", subject, audience, issued_at);
                let hash = sha256_hex(&[&seed]);
                hash[..32].to_string()
            });

            let token = Token {
                version: 1,
                token_id: token_id.clone(),
                subject: subject.clone(),
                audience: audience.clone(),
                capability: capability.clone(),
                issued_at,
                expires_at,
                signature: if allow_mock_signature { "mock".to_string() } else { "".to_string() },
            };

            write_json(&out, &token);

            if let Some(meta_out) = meta_out {
                let meta = json!({
                    "token_id": token_id,
                    "issued_at": issued_at,
                    "expires_at": expires_at,
                    "audience": audience,
                    "subject": subject,
                    "capability": capability,
                    "signature_mocked": allow_mock_signature
                });
                write_json(&meta_out, &meta);
            }
        }
    }
}
