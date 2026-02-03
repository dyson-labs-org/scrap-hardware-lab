#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use minicbor::{decode::Decoder, encode::Encoder};

pub const VERSION: u8 = 1;

pub const MSG_TASK_REQUEST: u8 = 1;
pub const MSG_TASK_RESULT: u8 = 2;
pub const MSG_TASK_REJECTED: u8 = 3;

const KEY_VERSION: u8 = 0;
const KEY_MSG_TYPE: u8 = 1;
const KEY_TRACE_ID: u8 = 2;
const KEY_SRC: u8 = 3;
const KEY_DST: u8 = 4;
const KEY_HOP_LIMIT: u8 = 5;
const KEY_PAYLOAD: u8 = 6;

const KEY_TOKEN: u8 = 0;
const KEY_COMMAND: u8 = 1;
const KEY_ARGS: u8 = 2;
const KEY_REPLY_TO: u8 = 3;
const KEY_COMMANDER: u8 = 4;

const KEY_STATUS: u8 = 0;
const KEY_OUTPUT_DIGEST: u8 = 1;
const KEY_TELEMETRY: u8 = 2;

const KEY_REASON: u8 = 0;
const KEY_DETAILS: u8 = 1;

const KEY_TOKEN_ID: u8 = 0;
const KEY_SUBJECT: u8 = 1;
const KEY_AUDIENCE: u8 = 2;
const KEY_CAPABILITY: u8 = 3;
const KEY_ISSUED_AT: u8 = 4;
const KEY_EXPIRES_AT: u8 = 5;

const KEY_TEL_DURATION_MS: u8 = 0;
const KEY_TEL_NODE_ID: u8 = 1;

pub const MAX_NODE_ID_LEN: usize = 32;
pub const MAX_PUBKEY_LEN: usize = 128;
pub const MAX_COMMAND_LEN: usize = 32;
pub const MAX_ARGS_LEN: usize = 64;
pub const MAX_REASON_LEN: usize = 64;
pub const MAX_DETAIL_LEN: usize = 64;
pub const MAX_DETAILS: usize = 8;
pub const TRACE_ID_LEN: usize = 16;
pub const TOKEN_ID_LEN: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteEntry {
    pub dst: String,
    pub next_hop: String,
}

#[derive(Clone, Debug)]
pub struct RouteTable {
    pub entries: Vec<RouteEntry>,
}

impl RouteTable {
    pub fn new(entries: Vec<RouteEntry>) -> Self {
        Self { entries }
    }

    pub fn next_hop(&self, dst: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.dst == dst)
            .map(|entry| entry.next_hop.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub token_id: Vec<u8>,
    pub subject: String,
    pub audience: String,
    pub capability: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRequest {
    pub token: Token,
    pub command: String,
    pub args: String,
    pub reply_to: String,
    pub commander_pubkey: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Telemetry {
    pub duration_ms: u32,
    pub node_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    pub status: u8,
    pub output_digest: Vec<u8>,
    pub telemetry: Telemetry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRejected {
    pub reason: String,
    pub details: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    TaskRequest(TaskRequest),
    TaskResult(TaskResult),
    TaskRejected(TaskRejected),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub version: u8,
    pub msg_type: u8,
    pub trace_id: Vec<u8>,
    pub src: String,
    pub dst: String,
    pub hop_limit: u8,
    pub payload: Payload,
}

#[derive(Debug)]
pub enum DecodeError {
    Cbor(minicbor::decode::Error),
    InvalidField(&'static str),
    LengthExceeded(&'static str),
}

impl From<minicbor::decode::Error> for DecodeError {
    fn from(err: minicbor::decode::Error) -> Self {
        DecodeError::Cbor(err)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Cbor(err) => write!(f, "cbor decode error: {err}"),
            DecodeError::InvalidField(field) => write!(f, "invalid field: {field}"),
            DecodeError::LengthExceeded(field) => write!(f, "length exceeded: {field}"),
        }
    }
}

#[derive(Debug)]
pub enum EncodeError {
    Cbor(minicbor::encode::Error<core::convert::Infallible>),
}

impl From<minicbor::encode::Error<core::convert::Infallible>> for EncodeError {
    fn from(err: minicbor::encode::Error<core::convert::Infallible>) -> Self {
        EncodeError::Cbor(err)
    }
}

fn decode_string(dec: &mut Decoder<'_>, max_len: usize) -> Result<String, DecodeError> {
    let value = dec.str()?;
    if value.len() > max_len {
        return Err(DecodeError::LengthExceeded("string"));
    }
    Ok(String::from(value))
}

fn decode_bytes(dec: &mut Decoder<'_>, max_len: usize) -> Result<Vec<u8>, DecodeError> {
    let value = dec.bytes()?;
    if value.len() > max_len {
        return Err(DecodeError::LengthExceeded("bytes"));
    }
    Ok(value.to_vec())
}

fn encode_string(enc: &mut Encoder<&mut Vec<u8>>, value: &str) -> Result<(), EncodeError> {
    enc.str(value)?;
    Ok(())
}

fn encode_bytes(enc: &mut Encoder<&mut Vec<u8>>, value: &[u8]) -> Result<(), EncodeError> {
    enc.bytes(value)?;
    Ok(())
}

pub fn encode_envelope(env: &Envelope, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    let mut enc = Encoder::new(out);
    enc.map(7)?;
    enc.u8(KEY_VERSION)?.u8(env.version)?;
    enc.u8(KEY_MSG_TYPE)?.u8(env.msg_type)?;
    enc.u8(KEY_TRACE_ID)?;
    encode_bytes(&mut enc, &env.trace_id)?;
    enc.u8(KEY_SRC)?;
    encode_string(&mut enc, &env.src)?;
    enc.u8(KEY_DST)?;
    encode_string(&mut enc, &env.dst)?;
    enc.u8(KEY_HOP_LIMIT)?.u8(env.hop_limit)?;
    enc.u8(KEY_PAYLOAD)?;

    match &env.payload {
        Payload::TaskRequest(task) => encode_task_request(&mut enc, task)?,
        Payload::TaskResult(result) => encode_task_result(&mut enc, result)?,
        Payload::TaskRejected(rejected) => encode_task_rejected(&mut enc, rejected)?,
    }

    Ok(())
}

pub fn decode_envelope(data: &[u8]) -> Result<Envelope, DecodeError> {
    let mut dec = Decoder::new(data);
    let mut version = None;
    let mut msg_type = None;
    let mut trace_id = None;
    let mut src = None;
    let mut dst = None;
    let mut hop_limit = None;
    let mut payload = None;

    let len = dec.map()?.unwrap_or(0);
    for _ in 0..len {
        let key = dec.u8()?;
        match key {
            KEY_VERSION => version = Some(dec.u8()?),
            KEY_MSG_TYPE => msg_type = Some(dec.u8()?),
            KEY_TRACE_ID => trace_id = Some(decode_bytes(&mut dec, TRACE_ID_LEN)?),
            KEY_SRC => src = Some(decode_string(&mut dec, MAX_NODE_ID_LEN)?),
            KEY_DST => dst = Some(decode_string(&mut dec, MAX_NODE_ID_LEN)?),
            KEY_HOP_LIMIT => hop_limit = Some(dec.u8()?),
            KEY_PAYLOAD => {
                let msg_type_val = msg_type.unwrap_or(0);
                payload = Some(match msg_type_val {
                    MSG_TASK_REQUEST => Payload::TaskRequest(decode_task_request(&mut dec)?),
                    MSG_TASK_RESULT => Payload::TaskResult(decode_task_result(&mut dec)?),
                    MSG_TASK_REJECTED => Payload::TaskRejected(decode_task_rejected(&mut dec)?),
                    _ => return Err(DecodeError::InvalidField("msg_type")),
                });
            }
            _ => {
                dec.skip()?;
            }
        }
    }

    Ok(Envelope {
        version: version.ok_or(DecodeError::InvalidField("version"))?,
        msg_type: msg_type.ok_or(DecodeError::InvalidField("msg_type"))?,
        trace_id: trace_id.ok_or(DecodeError::InvalidField("trace_id"))?,
        src: src.ok_or(DecodeError::InvalidField("src"))?,
        dst: dst.ok_or(DecodeError::InvalidField("dst"))?,
        hop_limit: hop_limit.ok_or(DecodeError::InvalidField("hop_limit"))?,
        payload: payload.ok_or(DecodeError::InvalidField("payload"))?,
    })
}

fn encode_task_request(enc: &mut Encoder<&mut Vec<u8>>, task: &TaskRequest) -> Result<(), EncodeError> {
    enc.map(5)?;
    enc.u8(KEY_TOKEN)?;
    encode_token(enc, &task.token)?;
    enc.u8(KEY_COMMAND)?;
    encode_string(enc, &task.command)?;
    enc.u8(KEY_ARGS)?;
    encode_string(enc, &task.args)?;
    enc.u8(KEY_REPLY_TO)?;
    encode_string(enc, &task.reply_to)?;
    enc.u8(KEY_COMMANDER)?;
    encode_string(enc, &task.commander_pubkey)?;
    Ok(())
}

fn decode_task_request(dec: &mut Decoder<'_>) -> Result<TaskRequest, DecodeError> {
    let len = dec.map()?.unwrap_or(0);
    let mut token = None;
    let mut command = None;
    let mut args = None;
    let mut reply_to = None;
    let mut commander = None;

    for _ in 0..len {
        let key = dec.u8()?;
        match key {
            KEY_TOKEN => token = Some(decode_token(dec)?),
            KEY_COMMAND => command = Some(decode_string(dec, MAX_COMMAND_LEN)?),
            KEY_ARGS => args = Some(decode_string(dec, MAX_ARGS_LEN)?),
            KEY_REPLY_TO => reply_to = Some(decode_string(dec, MAX_NODE_ID_LEN)?),
            KEY_COMMANDER => commander = Some(decode_string(dec, MAX_PUBKEY_LEN)?),
            _ => dec.skip()?,
        }
    }

    Ok(TaskRequest {
        token: token.ok_or(DecodeError::InvalidField("token"))?,
        command: command.ok_or(DecodeError::InvalidField("command"))?,
        args: args.ok_or(DecodeError::InvalidField("args"))?,
        reply_to: reply_to.ok_or(DecodeError::InvalidField("reply_to"))?,
        commander_pubkey: commander.ok_or(DecodeError::InvalidField("commander_pubkey"))?,
    })
}

fn encode_task_result(enc: &mut Encoder<&mut Vec<u8>>, result: &TaskResult) -> Result<(), EncodeError> {
    enc.map(3)?;
    enc.u8(KEY_STATUS)?.u8(result.status)?;
    enc.u8(KEY_OUTPUT_DIGEST)?;
    encode_bytes(enc, &result.output_digest)?;
    enc.u8(KEY_TELEMETRY)?;
    encode_telemetry(enc, &result.telemetry)?;
    Ok(())
}

fn decode_task_result(dec: &mut Decoder<'_>) -> Result<TaskResult, DecodeError> {
    let len = dec.map()?.unwrap_or(0);
    let mut status = None;
    let mut output = None;
    let mut telemetry = None;

    for _ in 0..len {
        let key = dec.u8()?;
        match key {
            KEY_STATUS => status = Some(dec.u8()?),
            KEY_OUTPUT_DIGEST => output = Some(decode_bytes(dec, 64)?),
            KEY_TELEMETRY => telemetry = Some(decode_telemetry(dec)?),
            _ => dec.skip()?,
        }
    }

    Ok(TaskResult {
        status: status.ok_or(DecodeError::InvalidField("status"))?,
        output_digest: output.ok_or(DecodeError::InvalidField("output_digest"))?,
        telemetry: telemetry.ok_or(DecodeError::InvalidField("telemetry"))?,
    })
}

fn encode_task_rejected(enc: &mut Encoder<&mut Vec<u8>>, rejected: &TaskRejected) -> Result<(), EncodeError> {
    enc.map(2)?;
    enc.u8(KEY_REASON)?;
    encode_string(enc, &rejected.reason)?;
    enc.u8(KEY_DETAILS)?;
    enc.array(rejected.details.len() as u64)?;
    for item in &rejected.details {
        encode_string(enc, item)?;
    }
    Ok(())
}

fn decode_task_rejected(dec: &mut Decoder<'_>) -> Result<TaskRejected, DecodeError> {
    let len = dec.map()?.unwrap_or(0);
    let mut reason = None;
    let mut details: Vec<String> = Vec::new();

    for _ in 0..len {
        let key = dec.u8()?;
        match key {
            KEY_REASON => reason = Some(decode_string(dec, MAX_REASON_LEN)?),
            KEY_DETAILS => {
                let arr_len = dec.array()?.unwrap_or(0);
                let capped = core::cmp::min(arr_len as usize, MAX_DETAILS);
                for _ in 0..capped {
                    details.push(decode_string(dec, MAX_DETAIL_LEN)?);
                }
                for _ in capped..arr_len as usize {
                    dec.skip()?;
                }
            }
            _ => dec.skip()?,
        }
    }

    Ok(TaskRejected {
        reason: reason.unwrap_or_else(|| String::from("validation_failed")),
        details,
    })
}

fn encode_token(enc: &mut Encoder<&mut Vec<u8>>, token: &Token) -> Result<(), EncodeError> {
    enc.map(6)?;
    enc.u8(KEY_TOKEN_ID)?;
    encode_bytes(enc, &token.token_id)?;
    enc.u8(KEY_SUBJECT)?;
    encode_string(enc, &token.subject)?;
    enc.u8(KEY_AUDIENCE)?;
    encode_string(enc, &token.audience)?;
    enc.u8(KEY_CAPABILITY)?;
    encode_string(enc, &token.capability)?;
    enc.u8(KEY_ISSUED_AT)?.u64(token.issued_at)?;
    enc.u8(KEY_EXPIRES_AT)?.u64(token.expires_at)?;
    Ok(())
}

fn decode_token(dec: &mut Decoder<'_>) -> Result<Token, DecodeError> {
    let len = dec.map()?.unwrap_or(0);
    let mut token_id = None;
    let mut subject = None;
    let mut audience = None;
    let mut capability = None;
    let mut issued_at = None;
    let mut expires_at = None;

    for _ in 0..len {
        let key = dec.u8()?;
        match key {
            KEY_TOKEN_ID => token_id = Some(decode_bytes(dec, TOKEN_ID_LEN)?),
            KEY_SUBJECT => subject = Some(decode_string(dec, MAX_PUBKEY_LEN)?),
            KEY_AUDIENCE => audience = Some(decode_string(dec, MAX_NODE_ID_LEN)?),
            KEY_CAPABILITY => capability = Some(decode_string(dec, MAX_COMMAND_LEN)?),
            KEY_ISSUED_AT => issued_at = Some(dec.u64()?),
            KEY_EXPIRES_AT => expires_at = Some(dec.u64()?),
            _ => dec.skip()?,
        }
    }

    Ok(Token {
        token_id: token_id.ok_or(DecodeError::InvalidField("token_id"))?,
        subject: subject.ok_or(DecodeError::InvalidField("subject"))?,
        audience: audience.ok_or(DecodeError::InvalidField("audience"))?,
        capability: capability.ok_or(DecodeError::InvalidField("capability"))?,
        issued_at: issued_at.ok_or(DecodeError::InvalidField("issued_at"))?,
        expires_at: expires_at.ok_or(DecodeError::InvalidField("expires_at"))?,
    })
}

fn encode_telemetry(enc: &mut Encoder<&mut Vec<u8>>, telemetry: &Telemetry) -> Result<(), EncodeError> {
    enc.map(2)?;
    enc.u8(KEY_TEL_DURATION_MS)?.u32(telemetry.duration_ms)?;
    enc.u8(KEY_TEL_NODE_ID)?;
    encode_string(enc, &telemetry.node_id)?;
    Ok(())
}

fn decode_telemetry(dec: &mut Decoder<'_>) -> Result<Telemetry, DecodeError> {
    let len = dec.map()?.unwrap_or(0);
    let mut duration_ms = None;
    let mut node_id = None;

    for _ in 0..len {
        let key = dec.u8()?;
        match key {
            KEY_TEL_DURATION_MS => duration_ms = Some(dec.u32()?),
            KEY_TEL_NODE_ID => node_id = Some(decode_string(dec, MAX_NODE_ID_LEN)?),
            _ => dec.skip()?,
        }
    }

    Ok(Telemetry {
        duration_ms: duration_ms.ok_or(DecodeError::InvalidField("duration_ms"))?,
        node_id: node_id.ok_or(DecodeError::InvalidField("node_id"))?,
    })
}

pub fn build_task_result(
    trace_id: Vec<u8>,
    src: String,
    dst: String,
    hop_limit: u8,
    result: TaskResult,
) -> Envelope {
    Envelope {
        version: VERSION,
        msg_type: MSG_TASK_RESULT,
        trace_id,
        src,
        dst,
        hop_limit,
        payload: Payload::TaskResult(result),
    }
}

pub fn build_task_rejected(
    trace_id: Vec<u8>,
    src: String,
    dst: String,
    hop_limit: u8,
    reason: String,
    details: Vec<String>,
) -> Envelope {
    Envelope {
        version: VERSION,
        msg_type: MSG_TASK_REJECTED,
        trace_id,
        src,
        dst,
        hop_limit,
        payload: Payload::TaskRejected(TaskRejected { reason, details }),
    }
}

pub fn build_task_request(
    trace_id: Vec<u8>,
    src: String,
    dst: String,
    hop_limit: u8,
    task: TaskRequest,
) -> Envelope {
    Envelope {
        version: VERSION,
        msg_type: MSG_TASK_REQUEST,
        trace_id,
        src,
        dst,
        hop_limit,
        payload: Payload::TaskRequest(task),
    }
}

impl Token {
    pub fn token_id_hex(&self) -> String {
        let mut out = String::with_capacity(self.token_id.len() * 2);
        for b in &self.token_id {
            use core::fmt::Write;
            let _ = write!(out, "{:02x}", b);
        }
        out
    }
}
