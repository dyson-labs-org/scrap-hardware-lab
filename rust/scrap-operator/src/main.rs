use clap::{Parser, Subcommand, ValueEnum};
use scrap_protocol::{
    bytes_to_hex, hex_to_bytes, keypair_from_secret, pubkey_from_secret, SpecOperator,
    SpecTokenCodec, TokenIssueRequest, Operator, TokenCodec,
};
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

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Mode {
    Demo,
    Spec,
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

        #[arg(long, value_enum, default_value = "demo")]
        mode: Mode,
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
            keys,
            out,
            meta_out,
            subject,
            audience,
            capability,
            expires_in,
            token_id,
            allow_mock_signature,
            mode,
        } => {
            let issued_at = unix_ts();
            let expires_at = issued_at + expires_in;
            match mode {
                Mode::Demo => {
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
                Mode::Spec => {
                    if allow_mock_signature {
                        panic!("spec mode does not allow mock signatures");
                    }
                    let keys_raw = fs::read_to_string(&keys).expect("keys read failed");
                    let keys_val: serde_json::Value =
                        serde_json::from_str(&keys_raw).expect("keys parse failed");
                    let operator_privkey = keys_val
                        .get("operator_privkey")
                        .and_then(|v| v.as_str())
                        .expect("keys missing operator_privkey");

                    let operator_key = keypair_from_secret(operator_privkey)
                        .expect("operator keypair failed");
                    let operator_pubkey =
                        pubkey_from_secret(operator_privkey).expect("operator pubkey failed");

                    let subject_bytes =
                        hex_to_bytes(&subject).unwrap_or_else(|_| subject.as_bytes().to_vec());
                    let audience_bytes = hex_to_bytes(&audience)
                        .expect("spec mode requires audience as hex pubkey or key-id");
                    if !matches!(audience_bytes.len(), 32 | 33) {
                        panic!("spec mode audience must be 32 or 33 bytes");
                    }

                    let token_id_bytes = match token_id {
                        Some(id) => {
                            let bytes = hex_to_bytes(&id).expect("token_id hex parse failed");
                            if bytes.len() != 16 {
                                panic!("token_id must be 16 bytes hex in spec mode");
                            }
                            let mut fixed = [0u8; 16];
                            fixed.copy_from_slice(&bytes);
                            Some(fixed)
                        }
                        None => None,
                    };

                    let request = TokenIssueRequest {
                        subject: subject_bytes,
                        audience: audience_bytes,
                        capability: vec![capability.clone()],
                        issued_at: issued_at as u32,
                        expires_at: expires_at as u32,
                        token_id: token_id_bytes,
                    };
                    let operator = SpecOperator {
                        operator_key,
                        operator_pubkey: operator_pubkey.clone(),
                    };
                    let token = operator.issue_token(&request).expect("spec token issue failed");
                    let codec = SpecTokenCodec;
                    let token_bytes = codec.encode_token(&token).expect("spec token encode failed");
                    fs::write(&out, &token_bytes).expect("spec token write failed");

                    if let Some(meta_out) = meta_out {
                        let meta = json!({
                            "token_id": bytes_to_hex(&token.token_id),
                            "issued_at": issued_at,
                            "expires_at": expires_at,
                            "issuer": bytes_to_hex(&operator_pubkey),
                            "audience": bytes_to_hex(&token.audience),
                            "subject": bytes_to_hex(&token.subject),
                            "capability": token.capabilities,
                            "token_tlv_hex": bytes_to_hex(&token_bytes)
                        });
                        write_json(&meta_out, &meta);
                    }
                }
            }
        }
    }
}
