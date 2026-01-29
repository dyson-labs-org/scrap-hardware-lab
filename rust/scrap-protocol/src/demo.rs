use crate::{MessageCodec, ProtocolError, TokenCodec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DemoToken {
    pub version: u8,
    pub token_id: String,
    pub subject: String,
    pub audience: String,
    pub capability: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DemoTaskRequest {
    pub version: u8,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub task_id: String,
    pub requested_capability: String,
    pub token: DemoToken,
    pub commander_pubkey: String,
    pub commander_signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DemoTaskAccepted {
    pub version: u8,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub task_id: String,
    pub payment_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DemoProof {
    pub version: u8,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub task_id: String,
    pub proof_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DemoTaskRejected {
    pub version: u8,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub task_id: String,
    pub details: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DemoMessage {
    TaskRequest(DemoTaskRequest),
    TaskAccepted(DemoTaskAccepted),
    Proof(DemoProof),
    TaskRejected(DemoTaskRejected),
}

#[derive(Debug, Default, Clone)]
pub struct DemoTokenCodec;

impl TokenCodec for DemoTokenCodec {
    type Token = DemoToken;

    fn encode_token(&self, token: &Self::Token) -> Result<Vec<u8>, ProtocolError> {
        serde_json::to_vec(token).map_err(|err| ProtocolError::new(err.to_string()))
    }

    fn decode_token(&self, bytes: &[u8]) -> Result<Self::Token, ProtocolError> {
        serde_json::from_slice(bytes).map_err(|err| ProtocolError::new(err.to_string()))
    }
}

#[derive(Debug, Default, Clone)]
pub struct DemoMessageCodec;

impl MessageCodec for DemoMessageCodec {
    type Message = DemoMessage;

    fn encode_message(&self, msg: &Self::Message) -> Result<Vec<u8>, ProtocolError> {
        let value = match msg {
            DemoMessage::TaskRequest(req) => serde_json::to_vec(req),
            DemoMessage::TaskAccepted(accept) => serde_json::to_vec(accept),
            DemoMessage::Proof(proof) => serde_json::to_vec(proof),
            DemoMessage::TaskRejected(reject) => serde_json::to_vec(reject),
        };
        value.map_err(|err| ProtocolError::new(err.to_string()))
    }

    fn decode_message(&self, bytes: &[u8]) -> Result<Self::Message, ProtocolError> {
        let value: Value =
            serde_json::from_slice(bytes).map_err(|err| ProtocolError::new(err.to_string()))?;
        let msg_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match msg_type {
            "task_request" => {
                let req: DemoTaskRequest =
                    serde_json::from_value(value).map_err(|err| ProtocolError::new(err.to_string()))?;
                Ok(DemoMessage::TaskRequest(req))
            }
            "task_accepted" => {
                let accept: DemoTaskAccepted =
                    serde_json::from_value(value).map_err(|err| ProtocolError::new(err.to_string()))?;
                Ok(DemoMessage::TaskAccepted(accept))
            }
            "proof" => {
                let proof: DemoProof =
                    serde_json::from_value(value).map_err(|err| ProtocolError::new(err.to_string()))?;
                Ok(DemoMessage::Proof(proof))
            }
            "task_rejected" => {
                let reject: DemoTaskRejected =
                    serde_json::from_value(value).map_err(|err| ProtocolError::new(err.to_string()))?;
                Ok(DemoMessage::TaskRejected(reject))
            }
            _ => Err(ProtocolError::new("demo: unknown message type")),
        }
    }
}
