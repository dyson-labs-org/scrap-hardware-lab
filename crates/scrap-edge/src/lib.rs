#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use scrap_core_lite::{
    build_task_rejected, Envelope, Payload, RouteTable, TaskRequest, TaskResult, Token,
    MAX_ARGS_LEN, MAX_COMMAND_LEN, MAX_NODE_ID_LEN, MAX_PUBKEY_LEN, MSG_TASK_REQUEST,
};

pub const DETAIL_SUBJECT_MISMATCH: &str = "token subject does not match commander_pubkey";
pub const DETAIL_REPLAY: &str = "replay detected (token_id already used)";

pub trait ReplayCache {
    fn check_and_add(&mut self, token_id: &[u8]) -> bool;
}

pub trait TokenVerifier {
    fn verify(
        &self,
        token: &Token,
        commander_pubkey: &str,
        expected_audience: &str,
        required_capability: &str,
        now: u64,
        revoked: &[Vec<u8>],
        expected_commander_pubkey: Option<&str>,
    ) -> Result<(), Vec<String>>;
}

#[derive(Debug)]
pub struct Context<'a, R: ReplayCache, V: TokenVerifier> {
    pub node_id: &'a str,
    pub routes: &'a RouteTable,
    pub replay: &'a mut R,
    pub revoked: &'a [Vec<u8>],
    pub commander_pubkey: Option<&'a str>,
    pub allow_mock_signatures: bool,
    pub verifier: &'a V,
}

#[derive(Debug)]
pub enum Action {
    Forward { next_hop: String, envelope: Envelope },
    Execute { task: TaskRequest, envelope: Envelope },
    Reply { envelope: Envelope },
    Drop,
}

pub fn handle_envelope<R: ReplayCache, V: TokenVerifier>(
    ctx: &mut Context<'_, R, V>,
    mut env: Envelope,
    now: u64,
) -> Action {
    if env.dst != ctx.node_id {
        if env.hop_limit == 0 {
            let reject = build_task_rejected(
                env.trace_id.clone(),
                ctx.node_id.to_string(),
                env.src.clone(),
                0,
                "hop_limit_exceeded".to_string(),
                vec!["hop limit exceeded".to_string()],
            );
            return Action::Reply { envelope: reject };
        }
        env.hop_limit = env.hop_limit.saturating_sub(1);
        if let Some(next_hop) = ctx.routes.next_hop(&env.dst) {
            return Action::Forward {
                next_hop: next_hop.to_string(),
                envelope: env,
            };
        }
        let reject = build_task_rejected(
            env.trace_id.clone(),
            ctx.node_id.to_string(),
            env.src.clone(),
            env.hop_limit,
            "no_route".to_string(),
            vec!["no route to destination".to_string()],
        );
        return Action::Reply { envelope: reject };
    }

    if env.msg_type != MSG_TASK_REQUEST {
        return Action::Drop;
    }

    let task = match env.payload.clone() {
        Payload::TaskRequest(task) => task,
        _ => return Action::Drop,
    };

    let mut details: Vec<String> = Vec::new();

    if task.command.len() > MAX_COMMAND_LEN || task.args.len() > MAX_ARGS_LEN {
        details.push("command or args too long".to_string());
    }

    if details.is_empty() {
        if let Err(mut issues) = ctx.verifier.verify(
            &task.token,
            &task.commander_pubkey,
            ctx.node_id,
            &task.command,
            now,
            ctx.revoked,
            ctx.commander_pubkey,
        ) {
            details.append(&mut issues);
        }
    }

    if details.is_empty() {
        if !ctx.replay.check_and_add(&task.token.token_id) {
            details.push(DETAIL_REPLAY.to_string());
        }
    }

    if !details.is_empty() {
        let reject = build_task_rejected(
            env.trace_id.clone(),
            ctx.node_id.to_string(),
            task.reply_to.clone(),
            env.hop_limit,
            "validation_failed".to_string(),
            details,
        );
        return Action::Reply { envelope: reject };
    }

    Action::Execute { task, envelope: env }
}

pub fn build_result_envelope(
    trace_id: Vec<u8>,
    src: String,
    dst: String,
    hop_limit: u8,
    status: u8,
    output_digest: Vec<u8>,
    duration_ms: u32,
) -> Envelope {
    let result = TaskResult {
        status,
        output_digest,
        telemetry: scrap_core_lite::Telemetry {
            duration_ms,
            node_id: src.clone(),
        },
    };
    scrap_core_lite::build_task_result(trace_id, src, dst, hop_limit, result)
}

pub fn parse_command(task: &TaskRequest) -> Option<(&str, u64)> {
    let cmd = task.command.as_str();
    if cmd == "demo.hash" || cmd == "demo.sleep" {
        let arg = task.args.parse::<u64>().ok()?;
        return Some((cmd, arg));
    }
    None
}

pub fn simple_digest(input: u64) -> Vec<u8> {
    let mut acc = input ^ 0xA5A5_A5A5_A5A5_A5A5u64;
    let mut out = Vec::with_capacity(32);
    for _ in 0..4 {
        acc = acc.wrapping_mul(6364136223846793005).wrapping_add(1);
        out.extend_from_slice(&acc.to_be_bytes());
    }
    out
}

pub fn validate_node_id(node_id: &str) -> bool {
    !node_id.is_empty() && node_id.len() <= MAX_NODE_ID_LEN
}

pub fn validate_token_subject(token: &Token) -> bool {
    !token.subject.is_empty() && token.subject.len() <= MAX_PUBKEY_LEN
}
