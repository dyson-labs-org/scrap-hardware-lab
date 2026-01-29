use crate::tlv::{decode_records, encode_records, TlvRecord};
use crate::{
    MessageCodec, Operator, ProtocolError, TokenCodec, TokenIssueRequest, Verifier, VerifyError,
};
use rand::rngs::OsRng;
use rand::RngCore;
use secp256k1::schnorr::Signature;
use secp256k1::{Keypair as KeyPair, Message, PublicKey, Secp256k1, SecretKey, XOnlyPublicKey};
use sha2::{Digest, Sha256};

pub const MSG_TASK_REQUEST: u8 = 0x01;
pub const MSG_TASK_ACCEPT: u8 = 0x02;
pub const MSG_TASK_REJECT: u8 = 0x03;
pub const MSG_PROOF_OF_EXECUTION: u8 = 0x04;
pub const MSG_PAYMENT_LOCK: u8 = 0x10;
pub const MSG_PAYMENT_CLAIM: u8 = 0x11;

const TLV_TOKEN_VERSION: u64 = 0;
const TLV_TOKEN_ISSUER: u64 = 2;
const TLV_TOKEN_SUBJECT: u64 = 4;
const TLV_TOKEN_AUDIENCE: u64 = 6;
const TLV_TOKEN_ISSUED_AT: u64 = 8;
const TLV_TOKEN_EXPIRES_AT: u64 = 10;
const TLV_TOKEN_ID: u64 = 12;
const TLV_TOKEN_CAPABILITY: u64 = 14;
const TLV_TOKEN_SIGNATURE: u64 = 240;

const TLV_TOKEN_CONSTRAINT_GEO: u64 = 13;
const TLV_TOKEN_CONSTRAINT_RATE: u64 = 15;
const TLV_TOKEN_CONSTRAINT_AMOUNT: u64 = 17;
const TLV_TOKEN_CONSTRAINT_AFTER: u64 = 19;

const TLV_TOKEN_ROOT_ISSUER: u64 = 20;
const TLV_TOKEN_ROOT_TOKEN_ID: u64 = 22;
const TLV_TOKEN_PARENT_TOKEN_ID: u64 = 24;
const TLV_TOKEN_CHAIN_DEPTH: u64 = 26;

const TLV_REQ_TASK_ID: u64 = 0;
const TLV_REQ_TIMESTAMP: u64 = 2;
const TLV_REQ_CAPABILITY_TOKEN: u64 = 4;
const TLV_REQ_DELEGATION_TOKEN: u64 = 6;
const TLV_REQ_TASK_TYPE: u64 = 8;
const TLV_REQ_TARGET: u64 = 10;
const TLV_REQ_PARAMETERS: u64 = 12;
const TLV_REQ_CONSTRAINTS: u64 = 14;
const TLV_REQ_MAX_AMOUNT_SATS: u64 = 16;
const TLV_REQ_TIMEOUT_BLOCKS: u64 = 18;
const TLV_REQ_COMMANDER_SIGNATURE: u64 = 20;

const TLV_ACCEPT_TASK_ID: u64 = 0;
const TLV_ACCEPT_TIMESTAMP: u64 = 2;
const TLV_ACCEPT_IN_REPLY_TO: u64 = 4;
const TLV_ACCEPT_PAYMENT_HASH: u64 = 6;
const TLV_ACCEPT_AMOUNT_SATS: u64 = 8;
const TLV_ACCEPT_EXPIRY_SEC: u64 = 10;
const TLV_ACCEPT_DESCRIPTION: u64 = 12;
const TLV_ACCEPT_EST_DURATION_SEC: u64 = 14;
const TLV_ACCEPT_EARLIEST_START: u64 = 16;
const TLV_ACCEPT_DATA_VOLUME_MB: u64 = 18;
const TLV_ACCEPT_QUALITY_ESTIMATE: u64 = 20;
const TLV_ACCEPT_EXECUTOR_SIGNATURE: u64 = 22;

const TLV_PROOF_TASK_ID: u64 = 0;
const TLV_PROOF_TOKEN_ID: u64 = 2;
const TLV_PROOF_PAYMENT_HASH: u64 = 4;
const TLV_PROOF_OUTPUT_HASH: u64 = 6;
const TLV_PROOF_EXECUTION_TS: u64 = 8;
const TLV_PROOF_EXECUTOR_PUBKEY: u64 = 10;
const TLV_PROOF_SIGNATURE: u64 = 12;

const TLV_LOCK_TASK_ID: u64 = 0;
const TLV_LOCK_CORRELATION_ID: u64 = 2;
const TLV_LOCK_PAYMENT_HASH: u64 = 4;
const TLV_LOCK_AMOUNT_SATS: u64 = 6;
const TLV_LOCK_TIMEOUT_BLOCKS: u64 = 8;
const TLV_LOCK_TIMESTAMP: u64 = 10;

const TLV_CLAIM_TASK_ID: u64 = 0;
const TLV_CLAIM_CORRELATION_ID: u64 = 2;
const TLV_CLAIM_PAYMENT_HASH: u64 = 4;
const TLV_CLAIM_PREIMAGE: u64 = 6;
const TLV_CLAIM_TIMESTAMP: u64 = 8;

const TLV_REJECT_TASK_ID: u64 = 0;
const TLV_REJECT_REASON: u64 = 2;
const TLV_REJECT_DETAILS: u64 = 4;
const TLV_REJECT_TIMESTAMP: u64 = 6;

#[derive(Debug, Clone, Default)]
pub struct SpecConstraints {
    pub geo: Option<String>,
    pub rate: Option<(u32, u32)>,
    pub amount: Option<u64>,
    pub not_before: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct SpecDelegation {
    pub root_issuer: Option<Vec<u8>>,
    pub root_token_id: Option<[u8; 16]>,
    pub parent_token_id: Option<[u8; 16]>,
    pub chain_depth: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct SpecToken {
    pub version: u8,
    pub issuer: Vec<u8>,
    pub subject: Vec<u8>,
    pub audience: Vec<u8>,
    pub issued_at: u32,
    pub expires_at: u32,
    pub token_id: [u8; 16],
    pub capabilities: Vec<String>,
    pub constraints: SpecConstraints,
    pub delegation: SpecDelegation,
    pub signature: [u8; 64],
}

#[derive(Debug, Clone)]
pub struct SpecTaskRequest {
    pub task_id: String,
    pub timestamp: u32,
    pub capability_token: Vec<u8>,
    pub delegation_chain: Vec<Vec<u8>>,
    pub task_type: String,
    pub target_json: String,
    pub parameters_json: String,
    pub constraints_json: String,
    pub payment_max_sats: u64,
    pub timeout_blocks: u32,
    pub commander_signature: [u8; 64],
}

#[derive(Debug, Clone)]
pub struct SpecTaskAccept {
    pub task_id: String,
    pub timestamp: u32,
    pub in_reply_to: [u8; 32],
    pub payment_hash: [u8; 32],
    pub amount_sats: u64,
    pub expiry_sec: u32,
    pub description: String,
    pub estimated_duration_sec: u32,
    pub earliest_start: u32,
    pub data_volume_mb: u32,
    pub quality_estimate: u32,
    pub executor_signature: [u8; 64],
}

#[derive(Debug, Clone)]
pub struct SpecProofOfExecution {
    pub task_id: String,
    pub task_token_id: [u8; 16],
    pub payment_hash: [u8; 32],
    pub output_hash: [u8; 32],
    pub execution_timestamp: u32,
    pub executor_pubkey: Vec<u8>,
    pub executor_signature: [u8; 64],
}

#[derive(Debug, Clone)]
pub struct SpecPaymentLock {
    pub task_id: String,
    pub correlation_id: [u8; 32],
    pub payment_hash: [u8; 32],
    pub amount_sats: u64,
    pub timeout_blocks: u32,
    pub timestamp: u32,
}

#[derive(Debug, Clone)]
pub struct SpecTaskReject {
    pub task_id: String,
    pub reason: String,
    pub details: String,
    pub timestamp: u32,
}

#[derive(Debug, Clone)]
pub struct SpecPaymentClaim {
    pub task_id: String,
    pub correlation_id: [u8; 32],
    pub payment_hash: [u8; 32],
    pub preimage: [u8; 32],
    pub timestamp: u32,
}

#[derive(Debug, Clone)]
pub enum SpecMessage {
    TaskRequest(SpecTaskRequest),
    TaskAccept(SpecTaskAccept),
    ProofOfExecution(SpecProofOfExecution),
    PaymentLock(SpecPaymentLock),
    PaymentClaim(SpecPaymentClaim),
    TaskReject(SpecTaskReject),
}

#[derive(Debug, Default, Clone)]
pub struct SpecTokenCodec;

impl TokenCodec for SpecTokenCodec {
    type Token = SpecToken;

    fn encode_token(&self, token: &Self::Token) -> Result<Vec<u8>, ProtocolError> {
        token.encode_tlv()
    }

    fn decode_token(&self, bytes: &[u8]) -> Result<Self::Token, ProtocolError> {
        SpecToken::decode_tlv(bytes)
    }
}

#[derive(Debug, Default, Clone)]
pub struct SpecMessageCodec;

impl MessageCodec for SpecMessageCodec {
    type Message = SpecMessage;

    fn encode_message(&self, msg: &Self::Message) -> Result<Vec<u8>, ProtocolError> {
        let (msg_type, body) = match msg {
            SpecMessage::TaskRequest(req) => (MSG_TASK_REQUEST, req.encode_tlv()?),
            SpecMessage::TaskAccept(accept) => (MSG_TASK_ACCEPT, accept.encode_tlv()?),
            SpecMessage::ProofOfExecution(proof) => (MSG_PROOF_OF_EXECUTION, proof.encode_tlv()?),
            SpecMessage::PaymentLock(lock) => (MSG_PAYMENT_LOCK, lock.encode_tlv()?),
            SpecMessage::PaymentClaim(claim) => (MSG_PAYMENT_CLAIM, claim.encode_tlv()?),
            SpecMessage::TaskReject(reject) => (MSG_TASK_REJECT, reject.encode_tlv()?),
        };
        encode_envelope(msg_type, &body)
    }

    fn decode_message(&self, bytes: &[u8]) -> Result<Self::Message, ProtocolError> {
        let (msg_type, body) = decode_envelope(bytes)?;
        match msg_type {
            MSG_TASK_REQUEST => Ok(SpecMessage::TaskRequest(SpecTaskRequest::decode_tlv(
                body,
            )?)),
            MSG_TASK_ACCEPT => Ok(SpecMessage::TaskAccept(SpecTaskAccept::decode_tlv(body)?)),
            MSG_PROOF_OF_EXECUTION => Ok(SpecMessage::ProofOfExecution(
                SpecProofOfExecution::decode_tlv(body)?,
            )),
            MSG_PAYMENT_LOCK => Ok(SpecMessage::PaymentLock(SpecPaymentLock::decode_tlv(body)?)),
            MSG_PAYMENT_CLAIM => Ok(SpecMessage::PaymentClaim(SpecPaymentClaim::decode_tlv(
                body,
            )?)),
            MSG_TASK_REJECT => Ok(SpecMessage::TaskReject(SpecTaskReject::decode_tlv(body)?)),
            _ => Err(ProtocolError::new("spec: unknown message type")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpecOperator {
    pub operator_key: KeyPair,
    pub operator_pubkey: Vec<u8>,
}

impl Operator for SpecOperator {
    type Token = SpecToken;
    type Accept = SpecTaskAccept;
    type Proof = SpecProofOfExecution;

    fn issue_token(&self, req: &TokenIssueRequest) -> Result<Self::Token, ProtocolError> {
        let token_id = match req.token_id {
            Some(id) => id,
            None => {
                let mut bytes = [0u8; 16];
                OsRng.fill_bytes(&mut bytes);
                bytes
            }
        };
        let mut token = SpecToken {
            version: 1,
            issuer: self.operator_pubkey.clone(),
            subject: req.subject.clone(),
            audience: req.audience.clone(),
            issued_at: req.issued_at,
            expires_at: req.expires_at,
            token_id,
            capabilities: req.capability.clone(),
            constraints: SpecConstraints::default(),
            delegation: SpecDelegation::default(),
            signature: [0u8; 64],
        };
        let signature = sign_tagged(
            "SCRAP/token/v1",
            &token.encode_tlv_without_signature()?,
            &self.operator_key,
        )?;
        token.signature = signature;
        Ok(token)
    }

    fn sign_accept(&self, accept: &Self::Accept) -> Result<[u8; 64], ProtocolError> {
        sign_tagged(
            "SCRAP/task_accept/v1",
            &accept.encode_tlv_without_signature()?,
            &self.operator_key,
        )
    }

    fn sign_proof(&self, proof_hash: [u8; 32]) -> Result<[u8; 64], ProtocolError> {
        sign_message_hash(proof_hash, &self.operator_key)
    }
}

#[derive(Debug, Clone)]
pub struct SpecVerifier {
    pub operator_pubkey: Vec<u8>,
    pub executor_pubkey: Vec<u8>,
}

impl Verifier for SpecVerifier {
    type Token = SpecToken;
    type Request = SpecTaskRequest;
    type Accept = SpecTaskAccept;
    type Proof = SpecProofOfExecution;

    fn verify_token(&self, token: &Self::Token, now: u64) -> Result<(), VerifyError> {
        verify_token_root(token, &self.operator_pubkey, &self.executor_pubkey, now)
    }

    fn verify_request(&self, request: &Self::Request, now: u64) -> Result<(), VerifyError> {
        let token = SpecToken::decode_tlv(&request.capability_token)
            .map_err(|err| VerifyError::new(err.reason))?;
        if !request.delegation_chain.is_empty() {
            let mut chain = Vec::new();
            for raw in &request.delegation_chain {
                let parsed =
                    SpecToken::decode_tlv(raw).map_err(|err| VerifyError::new(err.reason))?;
                chain.push(parsed);
            }
            verify_delegation_chain(
                &token,
                &chain,
                &self.operator_pubkey,
                &self.executor_pubkey,
                now,
            )?;
        } else {
            verify_token_root(&token, &self.operator_pubkey, &self.executor_pubkey, now)?;
        }
        let commander_pubkey = parse_xonly(&token.subject).map_err(|err| VerifyError::new(err))?;
        let signing_hash = request.commander_signing_hash();
        if !verify_schnorr(&signing_hash, &request.commander_signature, &commander_pubkey) {
            return Err(VerifyError::new("commander signature invalid"));
        }
        if !capabilities_subset(&[request.task_type.clone()], &token.capabilities) {
            return Err(VerifyError::new("capability not authorized"));
        }
        if request.task_type.is_empty() {
            return Err(VerifyError::new("task_type missing"));
        }
        Ok(())
    }

    fn verify_accept(
        &self,
        accept: &Self::Accept,
        expected_request_hash: [u8; 32],
    ) -> Result<(), VerifyError> {
        if accept.in_reply_to != expected_request_hash {
            return Err(VerifyError::new("task_accept in_reply_to mismatch"));
        }
        let executor_pubkey =
            parse_xonly(&self.executor_pubkey).map_err(|err| VerifyError::new(err))?;
        let signing_hash = accept.executor_signing_hash();
        if !verify_schnorr(&signing_hash, &accept.executor_signature, &executor_pubkey) {
            return Err(VerifyError::new("executor signature invalid"));
        }
        Ok(())
    }

    fn verify_proof(&self, proof: &Self::Proof) -> Result<(), VerifyError> {
        let executor_pubkey =
            parse_xonly(&proof.executor_pubkey).map_err(|err| VerifyError::new(err))?;
        let proof_hash = proof.proof_hash();
        if !verify_schnorr(&proof_hash, &proof.executor_signature, &executor_pubkey) {
            return Err(VerifyError::new("proof signature invalid"));
        }
        Ok(())
    }
}

impl SpecToken {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut records = self.base_records()?;
        records.push(TlvRecord {
            t: TLV_TOKEN_SIGNATURE,
            v: self.signature.to_vec(),
        });
        encode_records(&records)
    }

    pub fn encode_tlv_without_signature(&self) -> Result<Vec<u8>, ProtocolError> {
        let records = self.base_records()?;
        encode_records(&records)
    }

    fn base_records(&self) -> Result<Vec<TlvRecord>, ProtocolError> {
        let mut records = Vec::new();
        records.push(TlvRecord {
            t: TLV_TOKEN_VERSION,
            v: vec![self.version],
        });
        records.push(TlvRecord {
            t: TLV_TOKEN_ISSUER,
            v: self.issuer.clone(),
        });
        records.push(TlvRecord {
            t: TLV_TOKEN_SUBJECT,
            v: self.subject.clone(),
        });
        records.push(TlvRecord {
            t: TLV_TOKEN_AUDIENCE,
            v: self.audience.clone(),
        });
        records.push(TlvRecord {
            t: TLV_TOKEN_ISSUED_AT,
            v: self.issued_at.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_TOKEN_EXPIRES_AT,
            v: self.expires_at.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_TOKEN_ID,
            v: self.token_id.to_vec(),
        });
        for cap in &self.capabilities {
            records.push(TlvRecord {
                t: TLV_TOKEN_CAPABILITY,
                v: cap.as_bytes().to_vec(),
            });
        }
        if let Some(geo) = &self.constraints.geo {
            records.push(TlvRecord {
                t: TLV_TOKEN_CONSTRAINT_GEO,
                v: geo.as_bytes().to_vec(),
            });
        }
        if let Some((count, period)) = &self.constraints.rate {
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&count.to_be_bytes());
            bytes.extend_from_slice(&period.to_be_bytes());
            records.push(TlvRecord {
                t: TLV_TOKEN_CONSTRAINT_RATE,
                v: bytes,
            });
        }
        if let Some(amount) = self.constraints.amount {
            records.push(TlvRecord {
                t: TLV_TOKEN_CONSTRAINT_AMOUNT,
                v: amount.to_be_bytes().to_vec(),
            });
        }
        if let Some(after) = self.constraints.not_before {
            records.push(TlvRecord {
                t: TLV_TOKEN_CONSTRAINT_AFTER,
                v: after.to_be_bytes().to_vec(),
            });
        }
        if let Some(root) = &self.delegation.root_issuer {
            records.push(TlvRecord {
                t: TLV_TOKEN_ROOT_ISSUER,
                v: root.clone(),
            });
        }
        if let Some(root_id) = self.delegation.root_token_id {
            records.push(TlvRecord {
                t: TLV_TOKEN_ROOT_TOKEN_ID,
                v: root_id.to_vec(),
            });
        }
        if let Some(parent_id) = self.delegation.parent_token_id {
            records.push(TlvRecord {
                t: TLV_TOKEN_PARENT_TOKEN_ID,
                v: parent_id.to_vec(),
            });
        }
        if let Some(depth) = self.delegation.chain_depth {
            records.push(TlvRecord {
                t: TLV_TOKEN_CHAIN_DEPTH,
                v: vec![depth],
            });
        }
        Ok(records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut version: Option<u8> = None;
        let mut issuer: Option<Vec<u8>> = None;
        let mut subject: Option<Vec<u8>> = None;
        let mut audience: Option<Vec<u8>> = None;
        let mut issued_at: Option<u32> = None;
        let mut expires_at: Option<u32> = None;
        let mut token_id: Option<[u8; 16]> = None;
        let mut capabilities: Vec<String> = Vec::new();
        let mut signature: Option<[u8; 64]> = None;
        let mut constraints = SpecConstraints::default();
        let mut delegation = SpecDelegation::default();

        for record in records {
            match record.t {
                TLV_TOKEN_VERSION => {
                    version = record.v.first().copied();
                }
                TLV_TOKEN_ISSUER => {
                    issuer = Some(record.v);
                }
                TLV_TOKEN_SUBJECT => {
                    subject = Some(record.v);
                }
                TLV_TOKEN_AUDIENCE => {
                    audience = Some(record.v);
                }
                TLV_TOKEN_ISSUED_AT => {
                    issued_at = Some(read_u32(&record.v)?);
                }
                TLV_TOKEN_EXPIRES_AT => {
                    expires_at = Some(read_u32(&record.v)?);
                }
                TLV_TOKEN_ID => {
                    token_id = Some(read_fixed(&record.v)?);
                }
                TLV_TOKEN_CAPABILITY => {
                    let value = String::from_utf8(record.v)
                        .map_err(|_| ProtocolError::new("token capability not utf-8"))?;
                    capabilities.push(value);
                }
                TLV_TOKEN_SIGNATURE => {
                    signature = Some(read_fixed(&record.v)?);
                }
                TLV_TOKEN_CONSTRAINT_GEO => {
                    constraints.geo = Some(
                        String::from_utf8(record.v)
                            .map_err(|_| ProtocolError::new("constraint_geo not utf-8"))?,
                    );
                }
                TLV_TOKEN_CONSTRAINT_RATE => {
                    if record.v.len() != 8 {
                        return Err(ProtocolError::new("constraint_rate length invalid"));
                    }
                    let count =
                        u32::from_be_bytes([record.v[0], record.v[1], record.v[2], record.v[3]]);
                    let period =
                        u32::from_be_bytes([record.v[4], record.v[5], record.v[6], record.v[7]]);
                    constraints.rate = Some((count, period));
                }
                TLV_TOKEN_CONSTRAINT_AMOUNT => {
                    if record.v.len() != 8 {
                        return Err(ProtocolError::new("constraint_amount length invalid"));
                    }
                    let amount = u64::from_be_bytes([
                        record.v[0], record.v[1], record.v[2], record.v[3], record.v[4],
                        record.v[5], record.v[6], record.v[7],
                    ]);
                    constraints.amount = Some(amount);
                }
                TLV_TOKEN_CONSTRAINT_AFTER => {
                    constraints.not_before = Some(read_u32(&record.v)?);
                }
                TLV_TOKEN_ROOT_ISSUER => {
                    delegation.root_issuer = Some(record.v);
                }
                TLV_TOKEN_ROOT_TOKEN_ID => {
                    delegation.root_token_id = Some(read_fixed(&record.v)?);
                }
                TLV_TOKEN_PARENT_TOKEN_ID => {
                    delegation.parent_token_id = Some(read_fixed(&record.v)?);
                }
                TLV_TOKEN_CHAIN_DEPTH => {
                    delegation.chain_depth = record.v.first().copied();
                }
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("token: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecToken {
            version: version.ok_or_else(|| ProtocolError::new("token missing version"))?,
            issuer: issuer.ok_or_else(|| ProtocolError::new("token missing issuer"))?,
            subject: subject.ok_or_else(|| ProtocolError::new("token missing subject"))?,
            audience: audience.ok_or_else(|| ProtocolError::new("token missing audience"))?,
            issued_at: issued_at.ok_or_else(|| ProtocolError::new("token missing issued_at"))?,
            expires_at: expires_at.ok_or_else(|| ProtocolError::new("token missing expires_at"))?,
            token_id: token_id.ok_or_else(|| ProtocolError::new("token missing token_id"))?,
            capabilities,
            constraints,
            delegation,
            signature: signature.ok_or_else(|| ProtocolError::new("token missing signature"))?,
        })
    }
}

impl SpecTaskRequest {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut records = self.base_records()?;
        records.push(TlvRecord {
            t: TLV_REQ_COMMANDER_SIGNATURE,
            v: self.commander_signature.to_vec(),
        });
        encode_records(&records)
    }

    pub fn encode_tlv_without_signature(&self) -> Result<Vec<u8>, ProtocolError> {
        encode_records(&self.base_records()?)
    }

    fn base_records(&self) -> Result<Vec<TlvRecord>, ProtocolError> {
        let mut records = Vec::new();
        records.push(TlvRecord {
            t: TLV_REQ_TASK_ID,
            v: self.task_id.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_TIMESTAMP,
            v: self.timestamp.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_CAPABILITY_TOKEN,
            v: self.capability_token.clone(),
        });
        for token in &self.delegation_chain {
            records.push(TlvRecord {
                t: TLV_REQ_DELEGATION_TOKEN,
                v: token.clone(),
            });
        }
        records.push(TlvRecord {
            t: TLV_REQ_TASK_TYPE,
            v: self.task_type.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_TARGET,
            v: self.target_json.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_PARAMETERS,
            v: self.parameters_json.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_CONSTRAINTS,
            v: self.constraints_json.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_MAX_AMOUNT_SATS,
            v: self.payment_max_sats.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_REQ_TIMEOUT_BLOCKS,
            v: self.timeout_blocks.to_be_bytes().to_vec(),
        });
        Ok(records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut task_id: Option<String> = None;
        let mut timestamp: Option<u32> = None;
        let mut capability_token: Option<Vec<u8>> = None;
        let mut delegation_chain: Vec<Vec<u8>> = Vec::new();
        let mut task_type: Option<String> = None;
        let mut target_json: Option<String> = None;
        let mut parameters_json: Option<String> = None;
        let mut constraints_json: Option<String> = None;
        let mut payment_max_sats: Option<u64> = None;
        let mut timeout_blocks: Option<u32> = None;
        let mut commander_signature: Option<[u8; 64]> = None;

        for record in records {
            match record.t {
                TLV_REQ_TASK_ID => {
                    task_id = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_id not utf-8")
                    })?);
                }
                TLV_REQ_TIMESTAMP => timestamp = Some(read_u32(&record.v)?),
                TLV_REQ_CAPABILITY_TOKEN => capability_token = Some(record.v),
                TLV_REQ_DELEGATION_TOKEN => delegation_chain.push(record.v),
                TLV_REQ_TASK_TYPE => {
                    task_type = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_type not utf-8")
                    })?);
                }
                TLV_REQ_TARGET => {
                    target_json = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("target not utf-8")
                    })?);
                }
                TLV_REQ_PARAMETERS => {
                    parameters_json = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("parameters not utf-8")
                    })?);
                }
                TLV_REQ_CONSTRAINTS => {
                    constraints_json = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("constraints not utf-8")
                    })?);
                }
                TLV_REQ_MAX_AMOUNT_SATS => payment_max_sats = Some(read_u64(&record.v)?),
                TLV_REQ_TIMEOUT_BLOCKS => timeout_blocks = Some(read_u32(&record.v)?),
                TLV_REQ_COMMANDER_SIGNATURE => {
                    commander_signature = Some(read_fixed(&record.v)?);
                }
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("request: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecTaskRequest {
            task_id: task_id.ok_or_else(|| ProtocolError::new("request missing task_id"))?,
            timestamp: timestamp.ok_or_else(|| ProtocolError::new("request missing timestamp"))?,
            capability_token: capability_token
                .ok_or_else(|| ProtocolError::new("request missing capability_token"))?,
            delegation_chain,
            task_type: task_type.ok_or_else(|| ProtocolError::new("request missing task_type"))?,
            target_json: target_json.ok_or_else(|| ProtocolError::new("request missing target"))?,
            parameters_json: parameters_json
                .ok_or_else(|| ProtocolError::new("request missing parameters"))?,
            constraints_json: constraints_json
                .ok_or_else(|| ProtocolError::new("request missing constraints"))?,
            payment_max_sats: payment_max_sats
                .ok_or_else(|| ProtocolError::new("request missing max_amount"))?,
            timeout_blocks: timeout_blocks
                .ok_or_else(|| ProtocolError::new("request missing timeout_blocks"))?,
            commander_signature: commander_signature
                .ok_or_else(|| ProtocolError::new("request missing commander_signature"))?,
        })
    }

    pub fn request_hash(&self) -> [u8; 32] {
        sha256(&self.encode_tlv_without_signature().unwrap_or_default())
    }

    pub fn commander_signing_hash(&self) -> [u8; 32] {
        tagged_hash(
            "SCRAP/task_request/v1",
            &self.encode_tlv_without_signature().unwrap_or_default(),
        )
    }
}

impl SpecTaskAccept {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut records = self.base_records()?;
        records.push(TlvRecord {
            t: TLV_ACCEPT_EXECUTOR_SIGNATURE,
            v: self.executor_signature.to_vec(),
        });
        encode_records(&records)
    }

    pub fn encode_tlv_without_signature(&self) -> Result<Vec<u8>, ProtocolError> {
        encode_records(&self.base_records()?)
    }

    fn base_records(&self) -> Result<Vec<TlvRecord>, ProtocolError> {
        let mut records = Vec::new();
        records.push(TlvRecord {
            t: TLV_ACCEPT_TASK_ID,
            v: self.task_id.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_TIMESTAMP,
            v: self.timestamp.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_IN_REPLY_TO,
            v: self.in_reply_to.to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_PAYMENT_HASH,
            v: self.payment_hash.to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_AMOUNT_SATS,
            v: self.amount_sats.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_EXPIRY_SEC,
            v: self.expiry_sec.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_DESCRIPTION,
            v: self.description.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_EST_DURATION_SEC,
            v: self.estimated_duration_sec.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_EARLIEST_START,
            v: self.earliest_start.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_DATA_VOLUME_MB,
            v: self.data_volume_mb.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_ACCEPT_QUALITY_ESTIMATE,
            v: self.quality_estimate.to_be_bytes().to_vec(),
        });
        Ok(records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut task_id: Option<String> = None;
        let mut timestamp: Option<u32> = None;
        let mut in_reply_to: Option<[u8; 32]> = None;
        let mut payment_hash: Option<[u8; 32]> = None;
        let mut amount_sats: Option<u64> = None;
        let mut expiry_sec: Option<u32> = None;
        let mut description: Option<String> = None;
        let mut estimated_duration_sec: Option<u32> = None;
        let mut earliest_start: Option<u32> = None;
        let mut data_volume_mb: Option<u32> = None;
        let mut quality_estimate: Option<u32> = None;
        let mut executor_signature: Option<[u8; 64]> = None;

        for record in records {
            match record.t {
                TLV_ACCEPT_TASK_ID => {
                    task_id = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_id not utf-8")
                    })?);
                }
                TLV_ACCEPT_TIMESTAMP => timestamp = Some(read_u32(&record.v)?),
                TLV_ACCEPT_IN_REPLY_TO => in_reply_to = Some(read_fixed(&record.v)?),
                TLV_ACCEPT_PAYMENT_HASH => payment_hash = Some(read_fixed(&record.v)?),
                TLV_ACCEPT_AMOUNT_SATS => amount_sats = Some(read_u64(&record.v)?),
                TLV_ACCEPT_EXPIRY_SEC => expiry_sec = Some(read_u32(&record.v)?),
                TLV_ACCEPT_DESCRIPTION => {
                    description = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("description not utf-8")
                    })?);
                }
                TLV_ACCEPT_EST_DURATION_SEC => {
                    estimated_duration_sec = Some(read_u32(&record.v)?)
                }
                TLV_ACCEPT_EARLIEST_START => earliest_start = Some(read_u32(&record.v)?),
                TLV_ACCEPT_DATA_VOLUME_MB => data_volume_mb = Some(read_u32(&record.v)?),
                TLV_ACCEPT_QUALITY_ESTIMATE => quality_estimate = Some(read_u32(&record.v)?),
                TLV_ACCEPT_EXECUTOR_SIGNATURE => {
                    executor_signature = Some(read_fixed(&record.v)?)
                }
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("accept: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecTaskAccept {
            task_id: task_id.ok_or_else(|| ProtocolError::new("accept missing task_id"))?,
            timestamp: timestamp.ok_or_else(|| ProtocolError::new("accept missing timestamp"))?,
            in_reply_to: in_reply_to
                .ok_or_else(|| ProtocolError::new("accept missing in_reply_to"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("accept missing payment_hash"))?,
            amount_sats: amount_sats
                .ok_or_else(|| ProtocolError::new("accept missing amount_sats"))?,
            expiry_sec: expiry_sec.ok_or_else(|| ProtocolError::new("accept missing expiry_sec"))?,
            description: description
                .ok_or_else(|| ProtocolError::new("accept missing description"))?,
            estimated_duration_sec: estimated_duration_sec
                .ok_or_else(|| ProtocolError::new("accept missing estimated_duration_sec"))?,
            earliest_start: earliest_start
                .ok_or_else(|| ProtocolError::new("accept missing earliest_start"))?,
            data_volume_mb: data_volume_mb
                .ok_or_else(|| ProtocolError::new("accept missing data_volume_mb"))?,
            quality_estimate: quality_estimate
                .ok_or_else(|| ProtocolError::new("accept missing quality_estimate"))?,
            executor_signature: executor_signature
                .ok_or_else(|| ProtocolError::new("accept missing executor_signature"))?,
        })
    }

    pub fn executor_signing_hash(&self) -> [u8; 32] {
        tagged_hash(
            "SCRAP/task_accept/v1",
            &self.encode_tlv_without_signature().unwrap_or_default(),
        )
    }
}

impl SpecProofOfExecution {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut records = Vec::new();
        records.push(TlvRecord {
            t: TLV_PROOF_TASK_ID,
            v: self.task_id.as_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_PROOF_TOKEN_ID,
            v: self.task_token_id.to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_PROOF_PAYMENT_HASH,
            v: self.payment_hash.to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_PROOF_OUTPUT_HASH,
            v: self.output_hash.to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_PROOF_EXECUTION_TS,
            v: self.execution_timestamp.to_be_bytes().to_vec(),
        });
        records.push(TlvRecord {
            t: TLV_PROOF_EXECUTOR_PUBKEY,
            v: self.executor_pubkey.clone(),
        });
        records.push(TlvRecord {
            t: TLV_PROOF_SIGNATURE,
            v: self.executor_signature.to_vec(),
        });
        encode_records(&records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut task_id: Option<String> = None;
        let mut task_token_id: Option<[u8; 16]> = None;
        let mut payment_hash: Option<[u8; 32]> = None;
        let mut output_hash: Option<[u8; 32]> = None;
        let mut execution_timestamp: Option<u32> = None;
        let mut executor_pubkey: Option<Vec<u8>> = None;
        let mut executor_signature: Option<[u8; 64]> = None;

        for record in records {
            match record.t {
                TLV_PROOF_TASK_ID => {
                    task_id = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_id not utf-8")
                    })?);
                }
                TLV_PROOF_TOKEN_ID => task_token_id = Some(read_fixed(&record.v)?),
                TLV_PROOF_PAYMENT_HASH => payment_hash = Some(read_fixed(&record.v)?),
                TLV_PROOF_OUTPUT_HASH => output_hash = Some(read_fixed(&record.v)?),
                TLV_PROOF_EXECUTION_TS => execution_timestamp = Some(read_u32(&record.v)?),
                TLV_PROOF_EXECUTOR_PUBKEY => executor_pubkey = Some(record.v),
                TLV_PROOF_SIGNATURE => executor_signature = Some(read_fixed(&record.v)?),
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("proof: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecProofOfExecution {
            task_id: task_id.ok_or_else(|| ProtocolError::new("proof missing task_id"))?,
            task_token_id: task_token_id
                .ok_or_else(|| ProtocolError::new("proof missing token_id"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("proof missing payment_hash"))?,
            output_hash: output_hash
                .ok_or_else(|| ProtocolError::new("proof missing output_hash"))?,
            execution_timestamp: execution_timestamp
                .ok_or_else(|| ProtocolError::new("proof missing execution_timestamp"))?,
            executor_pubkey: executor_pubkey
                .ok_or_else(|| ProtocolError::new("proof missing executor_pubkey"))?,
            executor_signature: executor_signature
                .ok_or_else(|| ProtocolError::new("proof missing executor_signature"))?,
        })
    }

    pub fn proof_hash(&self) -> [u8; 32] {
        let mut msg = Vec::new();
        msg.extend_from_slice(&self.task_token_id);
        msg.extend_from_slice(&self.payment_hash);
        msg.extend_from_slice(&self.output_hash);
        msg.extend_from_slice(&self.execution_timestamp.to_be_bytes());
        tagged_hash("SCRAP/proof/v1", &msg)
    }
}

impl SpecPaymentLock {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let records = vec![
            TlvRecord {
                t: TLV_LOCK_TASK_ID,
                v: self.task_id.as_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_LOCK_CORRELATION_ID,
                v: self.correlation_id.to_vec(),
            },
            TlvRecord {
                t: TLV_LOCK_PAYMENT_HASH,
                v: self.payment_hash.to_vec(),
            },
            TlvRecord {
                t: TLV_LOCK_AMOUNT_SATS,
                v: self.amount_sats.to_be_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_LOCK_TIMEOUT_BLOCKS,
                v: self.timeout_blocks.to_be_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_LOCK_TIMESTAMP,
                v: self.timestamp.to_be_bytes().to_vec(),
            },
        ];
        encode_records(&records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut task_id: Option<String> = None;
        let mut correlation_id: Option<[u8; 32]> = None;
        let mut payment_hash: Option<[u8; 32]> = None;
        let mut amount_sats: Option<u64> = None;
        let mut timeout_blocks: Option<u32> = None;
        let mut timestamp: Option<u32> = None;

        for record in records {
            match record.t {
                TLV_LOCK_TASK_ID => {
                    task_id = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_id not utf-8")
                    })?);
                }
                TLV_LOCK_CORRELATION_ID => correlation_id = Some(read_fixed(&record.v)?),
                TLV_LOCK_PAYMENT_HASH => payment_hash = Some(read_fixed(&record.v)?),
                TLV_LOCK_AMOUNT_SATS => amount_sats = Some(read_u64(&record.v)?),
                TLV_LOCK_TIMEOUT_BLOCKS => timeout_blocks = Some(read_u32(&record.v)?),
                TLV_LOCK_TIMESTAMP => timestamp = Some(read_u32(&record.v)?),
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("lock: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecPaymentLock {
            task_id: task_id.ok_or_else(|| ProtocolError::new("lock missing task_id"))?,
            correlation_id: correlation_id
                .ok_or_else(|| ProtocolError::new("lock missing correlation_id"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("lock missing payment_hash"))?,
            amount_sats: amount_sats
                .ok_or_else(|| ProtocolError::new("lock missing amount_sats"))?,
            timeout_blocks: timeout_blocks
                .ok_or_else(|| ProtocolError::new("lock missing timeout_blocks"))?,
            timestamp: timestamp.ok_or_else(|| ProtocolError::new("lock missing timestamp"))?,
        })
    }
}

impl SpecPaymentClaim {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let records = vec![
            TlvRecord {
                t: TLV_CLAIM_TASK_ID,
                v: self.task_id.as_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_CLAIM_CORRELATION_ID,
                v: self.correlation_id.to_vec(),
            },
            TlvRecord {
                t: TLV_CLAIM_PAYMENT_HASH,
                v: self.payment_hash.to_vec(),
            },
            TlvRecord {
                t: TLV_CLAIM_PREIMAGE,
                v: self.preimage.to_vec(),
            },
            TlvRecord {
                t: TLV_CLAIM_TIMESTAMP,
                v: self.timestamp.to_be_bytes().to_vec(),
            },
        ];
        encode_records(&records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut task_id: Option<String> = None;
        let mut correlation_id: Option<[u8; 32]> = None;
        let mut payment_hash: Option<[u8; 32]> = None;
        let mut preimage: Option<[u8; 32]> = None;
        let mut timestamp: Option<u32> = None;

        for record in records {
            match record.t {
                TLV_CLAIM_TASK_ID => {
                    task_id = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_id not utf-8")
                    })?);
                }
                TLV_CLAIM_CORRELATION_ID => correlation_id = Some(read_fixed(&record.v)?),
                TLV_CLAIM_PAYMENT_HASH => payment_hash = Some(read_fixed(&record.v)?),
                TLV_CLAIM_PREIMAGE => preimage = Some(read_fixed(&record.v)?),
                TLV_CLAIM_TIMESTAMP => timestamp = Some(read_u32(&record.v)?),
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("claim: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecPaymentClaim {
            task_id: task_id.ok_or_else(|| ProtocolError::new("claim missing task_id"))?,
            correlation_id: correlation_id
                .ok_or_else(|| ProtocolError::new("claim missing correlation_id"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("claim missing payment_hash"))?,
            preimage: preimage.ok_or_else(|| ProtocolError::new("claim missing preimage"))?,
            timestamp: timestamp.ok_or_else(|| ProtocolError::new("claim missing timestamp"))?,
        })
    }
}

impl SpecTaskReject {
    pub fn encode_tlv(&self) -> Result<Vec<u8>, ProtocolError> {
        let records = vec![
            TlvRecord {
                t: TLV_REJECT_TASK_ID,
                v: self.task_id.as_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_REJECT_REASON,
                v: self.reason.as_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_REJECT_DETAILS,
                v: self.details.as_bytes().to_vec(),
            },
            TlvRecord {
                t: TLV_REJECT_TIMESTAMP,
                v: self.timestamp.to_be_bytes().to_vec(),
            },
        ];
        encode_records(&records)
    }

    pub fn decode_tlv(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let records = decode_records(bytes)?;
        let mut task_id: Option<String> = None;
        let mut reason: Option<String> = None;
        let mut details: Option<String> = None;
        let mut timestamp: Option<u32> = None;

        for record in records {
            match record.t {
                TLV_REJECT_TASK_ID => {
                    task_id = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("task_id not utf-8")
                    })?);
                }
                TLV_REJECT_REASON => {
                    reason = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("reason not utf-8")
                    })?);
                }
                TLV_REJECT_DETAILS => {
                    details = Some(String::from_utf8(record.v).map_err(|_| {
                        ProtocolError::new("details not utf-8")
                    })?);
                }
                TLV_REJECT_TIMESTAMP => timestamp = Some(read_u32(&record.v)?),
                _ => {
                    if record.t % 2 == 0 {
                        return Err(ProtocolError::new("reject: unknown even tlv type"));
                    }
                }
            }
        }

        Ok(SpecTaskReject {
            task_id: task_id.ok_or_else(|| ProtocolError::new("reject missing task_id"))?,
            reason: reason.ok_or_else(|| ProtocolError::new("reject missing reason"))?,
            details: details.ok_or_else(|| ProtocolError::new("reject missing details"))?,
            timestamp: timestamp.ok_or_else(|| ProtocolError::new("reject missing timestamp"))?,
        })
    }
}

pub fn encode_envelope(msg_type: u8, body: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if body.len() > u16::MAX as usize {
        return Err(ProtocolError::new("envelope body too large"));
    }
    let mut out = Vec::with_capacity(3 + body.len());
    out.push(msg_type);
    out.extend_from_slice(&(body.len() as u16).to_be_bytes());
    out.extend_from_slice(body);
    Ok(out)
}

pub fn decode_envelope(bytes: &[u8]) -> Result<(u8, &[u8]), ProtocolError> {
    if bytes.len() < 3 {
        return Err(ProtocolError::new("envelope too short"));
    }
    let msg_type = bytes[0];
    let len = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
    if bytes.len() - 3 != len {
        return Err(ProtocolError::new("envelope length mismatch"));
    }
    Ok((msg_type, &bytes[3..]))
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result[..]);
    out
}

pub fn tagged_hash(tag: &str, msg: &[u8]) -> [u8; 32] {
    let tag_hash = sha256(tag.as_bytes());
    let mut buf = Vec::with_capacity(tag_hash.len() * 2 + msg.len());
    buf.extend_from_slice(&tag_hash);
    buf.extend_from_slice(&tag_hash);
    buf.extend_from_slice(msg);
    sha256(&buf)
}

pub fn derive_preimage(correlation_id: [u8; 32]) -> [u8; 32] {
    tagged_hash("SCRAP/preimage/v1", &correlation_id)
}

pub fn derive_payment_hash(correlation_id: [u8; 32]) -> [u8; 32] {
    sha256(&derive_preimage(correlation_id))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementPhase {
    Requested,
    Locked,
    Accepted,
    ProofSent,
    Claimed,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct SettlementState {
    pub task_id: String,
    pub correlation_id: [u8; 32],
    pub request_timestamp: u32,
    pub max_amount_sats: u64,
    pub timeout_blocks: u32,
    pub phase: SettlementPhase,
}

impl SettlementState {
    pub fn new(request: &SpecTaskRequest) -> Self {
        Self {
            task_id: request.task_id.clone(),
            correlation_id: request.request_hash(),
            request_timestamp: request.timestamp,
            max_amount_sats: request.payment_max_sats,
            timeout_blocks: request.timeout_blocks,
            phase: SettlementPhase::Requested,
        }
    }

    pub fn validate_lock(&self, lock: &SpecPaymentLock, now: u64) -> Result<(), VerifyError> {
        if lock.task_id != self.task_id {
            return Err(VerifyError::new("lock task_id mismatch"));
        }
        if lock.correlation_id != self.correlation_id {
            return Err(VerifyError::new("lock correlation_id mismatch"));
        }
        if lock.amount_sats > self.max_amount_sats {
            return Err(VerifyError::new("lock amount exceeds offer"));
        }
        if lock.timeout_blocks != self.timeout_blocks {
            return Err(VerifyError::new("lock timeout mismatch"));
        }
        let expires_at = self.request_timestamp as u64 + self.timeout_blocks as u64;
        if now > expires_at {
            return Err(VerifyError::new("lock timeout elapsed"));
        }
        let expected_hash = derive_payment_hash(self.correlation_id);
        if lock.payment_hash != expected_hash {
            return Err(VerifyError::new("lock payment_hash mismatch"));
        }
        Ok(())
    }

    pub fn mark_locked(&mut self) {
        self.phase = SettlementPhase::Locked;
    }

    pub fn can_emit_accept(&self) -> bool {
        self.phase == SettlementPhase::Locked
    }

    pub fn can_emit_proof(&self) -> bool {
        matches!(self.phase, SettlementPhase::Accepted | SettlementPhase::Locked)
    }
}

pub fn parse_secret_key(hex: &str) -> Result<SecretKey, ProtocolError> {
    let bytes = hex_to_bytes(hex)?;
    SecretKey::from_slice(&bytes).map_err(|_| ProtocolError::new("invalid secret key"))
}

pub fn keypair_from_secret(hex: &str) -> Result<KeyPair, ProtocolError> {
    let secp = Secp256k1::new();
    let secret = parse_secret_key(hex)?;
    Ok(KeyPair::from_secret_key(&secp, &secret))
}

pub fn pubkey_from_secret(hex: &str) -> Result<Vec<u8>, ProtocolError> {
    let secp = Secp256k1::new();
    let secret = parse_secret_key(hex)?;
    let public = PublicKey::from_secret_key(&secp, &secret);
    Ok(public.serialize().to_vec())
}

pub fn xonly_from_secret(hex: &str) -> Result<XOnlyPublicKey, ProtocolError> {
    let secp = Secp256k1::new();
    let secret = parse_secret_key(hex)?;
    let keypair = KeyPair::from_secret_key(&secp, &secret);
    Ok(XOnlyPublicKey::from_keypair(&keypair).0)
}

pub fn parse_xonly(bytes: &[u8]) -> Result<XOnlyPublicKey, String> {
    match bytes.len() {
        32 => XOnlyPublicKey::from_slice(bytes).map_err(|_| "invalid x-only pubkey".to_string()),
        33 => {
            let pubkey =
                PublicKey::from_slice(bytes).map_err(|_| "invalid compressed pubkey".to_string())?;
            Ok(pubkey.x_only_public_key().0)
        }
        _ => Err("invalid pubkey length".to_string()),
    }
}

pub fn normalize_pubkey(bytes: &[u8]) -> Result<[u8; 32], ProtocolError> {
    let xonly = parse_xonly(bytes).map_err(ProtocolError::new)?;
    Ok(xonly.serialize())
}

pub fn sign_tagged(tag: &str, data: &[u8], keypair: &KeyPair) -> Result<[u8; 64], ProtocolError> {
    let hash = tagged_hash(tag, data);
    sign_message_hash(hash, keypair)
}

pub fn sign_message_hash(hash: [u8; 32], keypair: &KeyPair) -> Result<[u8; 64], ProtocolError> {
    let secp = Secp256k1::new();
    let msg = Message::from_slice(&hash).map_err(|_| ProtocolError::new("invalid message hash"))?;
    let sig = secp.sign_schnorr(&msg, keypair);
    Ok(*sig.as_ref())
}

pub fn verify_schnorr(hash: &[u8; 32], signature: &[u8; 64], pubkey: &XOnlyPublicKey) -> bool {
    let secp = Secp256k1::verification_only();
    let msg = match Message::from_slice(hash) {
        Ok(msg) => msg,
        Err(_) => return false,
    };
    let sig = match Signature::from_slice(signature) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    secp.verify_schnorr(&sig, &msg, pubkey).is_ok()
}

pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ProtocolError> {
    let hex = hex.trim_start_matches("0x");
    if hex.len() % 2 != 0 {
        return Err(ProtocolError::new("hex string has odd length"));
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let chars: Vec<char> = hex.chars().collect();
    for i in (0..chars.len()).step_by(2) {
        let hi = chars[i].to_digit(16).ok_or_else(|| ProtocolError::new("invalid hex"))?;
        let lo = chars[i + 1]
            .to_digit(16)
            .ok_or_else(|| ProtocolError::new("invalid hex"))?;
        out.push(((hi << 4) + lo) as u8);
    }
    Ok(out)
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn read_fixed<const N: usize>(bytes: &[u8]) -> Result<[u8; N], ProtocolError> {
    if bytes.len() != N {
        return Err(ProtocolError::new("invalid fixed-size field"));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(bytes);
    Ok(out)
}

fn read_u32(bytes: &[u8]) -> Result<u32, ProtocolError> {
    if bytes.len() != 4 {
        return Err(ProtocolError::new("invalid u32 field"));
    }
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(bytes: &[u8]) -> Result<u64, ProtocolError> {
    if bytes.len() != 8 {
        return Err(ProtocolError::new("invalid u64 field"));
    }
    Ok(u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn verify_token_root(
    token: &SpecToken,
    operator_pubkey: &[u8],
    executor_pubkey: &[u8],
    now: u64,
) -> Result<(), VerifyError> {
    let op_xonly = parse_xonly(operator_pubkey).map_err(VerifyError::new)?;
    let token_hash = tagged_hash(
        "SCRAP/token/v1",
        &token.encode_tlv_without_signature().unwrap_or_default(),
    );
    if !verify_schnorr(&token_hash, &token.signature, &op_xonly) {
        return Err(VerifyError::new("token signature invalid"));
    }
    let issuer_norm =
        normalize_pubkey(&token.issuer).map_err(|_| VerifyError::new("issuer not pubkey"))?;
    let operator_norm =
        normalize_pubkey(operator_pubkey).map_err(|_| VerifyError::new("operator pubkey invalid"))?;
    if issuer_norm != operator_norm {
        return Err(VerifyError::new("token issuer mismatch"));
    }
    if !audience_matches(&token.audience, executor_pubkey)? {
        return Err(VerifyError::new("token audience mismatch"));
    }
    if token.expires_at as u64 <= now {
        return Err(VerifyError::new("token expired"));
    }
    Ok(())
}

fn verify_delegation_chain(
    leaf: &SpecToken,
    chain: &[SpecToken],
    operator_pubkey: &[u8],
    executor_pubkey: &[u8],
    now: u64,
) -> Result<(), VerifyError> {
    let mut full_chain = Vec::with_capacity(chain.len() + 1);
    full_chain.extend_from_slice(chain);
    full_chain.push(leaf.clone());

    let op_xonly = parse_xonly(operator_pubkey).map_err(VerifyError::new)?;
    let root = &full_chain[0];
    let root_hash = tagged_hash(
        "SCRAP/token/v1",
        &root.encode_tlv_without_signature().unwrap_or_default(),
    );
    if !verify_schnorr(&root_hash, &root.signature, &op_xonly) {
        return Err(VerifyError::new("root token signature invalid"));
    }
    let root_issuer_norm =
        normalize_pubkey(&root.issuer).map_err(|_| VerifyError::new("root issuer not pubkey"))?;
    let operator_norm =
        normalize_pubkey(operator_pubkey).map_err(|_| VerifyError::new("operator pubkey invalid"))?;
    if root_issuer_norm != operator_norm {
        return Err(VerifyError::new("root issuer mismatch"));
    }
    if root.chain_depth_mismatch() {
        return Err(VerifyError::new("root chain depth invalid"));
    }
    if !audience_matches(&root.audience, executor_pubkey)? {
        return Err(VerifyError::new("root audience mismatch"));
    }
    if root.expires_at as u64 <= now {
        return Err(VerifyError::new("root token expired"));
    }

    for i in 1..full_chain.len() {
        let parent = &full_chain[i - 1];
        let child = &full_chain[i];
        if child.issuer != parent.subject {
            return Err(VerifyError::new("delegation issuer mismatch"));
        }
        let parent_xonly = parse_xonly(&parent.subject).map_err(VerifyError::new)?;
        let child_hash = tagged_hash(
            "SCRAP/delegation/v1",
            &child.encode_tlv_without_signature().unwrap_or_default(),
        );
        if !verify_schnorr(&child_hash, &child.signature, &parent_xonly) {
            return Err(VerifyError::new("delegation signature invalid"));
        }
        if child.expires_at > parent.expires_at {
            return Err(VerifyError::new("delegation extends expiration"));
        }
        if !capabilities_subset(&child.capabilities, &parent.capabilities) {
            return Err(VerifyError::new("delegation capability not subset"));
        }
        let expected_depth = parent.delegation.chain_depth.unwrap_or(0) + 1;
        if child.delegation.chain_depth != Some(expected_depth) {
            return Err(VerifyError::new("delegation depth mismatch"));
        }
        if child.delegation.root_issuer.as_deref() != Some(operator_pubkey) {
            return Err(VerifyError::new("delegation root issuer mismatch"));
        }
        if child.delegation.root_token_id != Some(root.token_id) {
            return Err(VerifyError::new("delegation root token id mismatch"));
        }
        if child.expires_at as u64 <= now {
            return Err(VerifyError::new("delegation token expired"));
        }
    }
    Ok(())
}

impl SpecToken {
    fn chain_depth_mismatch(&self) -> bool {
        matches!(self.delegation.chain_depth, Some(depth) if depth != 0)
    }
}

fn audience_matches(audience: &[u8], executor_pubkey: &[u8]) -> Result<bool, VerifyError> {
    let executor_norm = normalize_pubkey(executor_pubkey)
        .map_err(|_| VerifyError::new("executor pubkey invalid"))?;
    if let Ok(aud_norm) = normalize_pubkey(audience) {
        if aud_norm == executor_norm {
            return Ok(true);
        }
    }
    let key_id = sha256(&executor_norm);
    Ok(audience == &key_id[..])
}

fn capabilities_subset(child: &[String], parent: &[String]) -> bool {
    child.iter().all(|cap| parent.iter().any(|p| capability_allows(p, cap)))
}

fn capability_allows(parent: &str, child: &str) -> bool {
    if parent == child {
        return true;
    }
    if let Some(prefix) = parent.strip_suffix('*') {
        return child.starts_with(prefix);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair() -> KeyPair {
        let secp = Secp256k1::new();
        let secret = SecretKey::new(&mut OsRng);
        KeyPair::from_secret_key(&secp, &secret)
    }

    #[test]
    fn token_roundtrip_and_verify() {
        let operator = keypair();
        let commander = keypair();
        let executor = keypair();
        let operator_pub = PublicKey::from_keypair(&operator).serialize().to_vec();
        let executor_pub = PublicKey::from_keypair(&executor).serialize().to_vec();
        let commander_pub = PublicKey::from_keypair(&commander).serialize().to_vec();

        let issue = TokenIssueRequest {
            subject: commander_pub.clone(),
            audience: executor_pub.clone(),
            capability: vec!["cmd:imaging:msi".to_string()],
            issued_at: 1,
            expires_at: 100,
            token_id: None,
        };
        let operator_impl = SpecOperator {
            operator_key: operator,
            operator_pubkey: operator_pub.clone(),
        };
        let token = operator_impl.issue_token(&issue).expect("issue token");
        let encoded = token.encode_tlv().expect("encode token");
        let decoded = SpecToken::decode_tlv(&encoded).expect("decode token");
        assert_eq!(decoded.token_id, token.token_id);
        let verifier = SpecVerifier {
            operator_pubkey: operator_pub,
            executor_pubkey: executor_pub,
        };
        verifier.verify_token(&decoded, 10).expect("verify token");
    }

    #[test]
    fn token_audience_mismatch_rejected() {
        let operator = keypair();
        let commander = keypair();
        let executor_a = keypair();
        let executor_b = keypair();

        let operator_pub = PublicKey::from_keypair(&operator).serialize().to_vec();
        let executor_a_pub = PublicKey::from_keypair(&executor_a).serialize().to_vec();
        let executor_b_pub = PublicKey::from_keypair(&executor_b).serialize().to_vec();
        let commander_pub = PublicKey::from_keypair(&commander).serialize().to_vec();

        let issue = TokenIssueRequest {
            subject: commander_pub,
            audience: executor_a_pub,
            capability: vec!["cmd:imaging:msi".to_string()],
            issued_at: 1,
            expires_at: 100,
            token_id: None,
        };
        let operator_impl = SpecOperator {
            operator_key: operator,
            operator_pubkey: operator_pub.clone(),
        };
        let token = operator_impl.issue_token(&issue).expect("issue token");
        let verifier = SpecVerifier {
            operator_pubkey: operator_pub,
            executor_pubkey: executor_b_pub,
        };
        assert!(verifier.verify_token(&token, 10).is_err());
    }

    #[test]
    fn token_audience_key_id_accepted() {
        let operator = keypair();
        let commander = keypair();
        let executor = keypair();

        let operator_pub = PublicKey::from_keypair(&operator).serialize().to_vec();
        let executor_pub = PublicKey::from_keypair(&executor).serialize().to_vec();
        let commander_pub = PublicKey::from_keypair(&commander).serialize().to_vec();

        let executor_norm = normalize_pubkey(&executor_pub).expect("normalize executor");
        let key_id = sha256(&executor_norm);

        let issue = TokenIssueRequest {
            subject: commander_pub,
            audience: key_id.to_vec(),
            capability: vec!["cmd:imaging:msi".to_string()],
            issued_at: 1,
            expires_at: 100,
            token_id: None,
        };
        let operator_impl = SpecOperator {
            operator_key: operator,
            operator_pubkey: operator_pub.clone(),
        };
        let token = operator_impl.issue_token(&issue).expect("issue token");
        let verifier = SpecVerifier {
            operator_pubkey: operator_pub,
            executor_pubkey: executor_pub,
        };
        verifier.verify_token(&token, 10).expect("verify token");
    }

    #[test]
    fn request_signing_and_hash() {
        let commander = keypair();
        let token_bytes = vec![0x01, 0x02];
        let mut request = SpecTaskRequest {
            task_id: "task-1".to_string(),
            timestamp: 10,
            capability_token: token_bytes,
            delegation_chain: vec![],
            task_type: "cmd:imaging:msi".to_string(),
            target_json: "{}".to_string(),
            parameters_json: "{}".to_string(),
            constraints_json: "{}".to_string(),
            payment_max_sats: 1000,
            timeout_blocks: 144,
            commander_signature: [0u8; 64],
        };
        let hash = request.commander_signing_hash();
        let sig = sign_message_hash(hash, &commander).expect("sign");
        request.commander_signature = sig;
        let encoded = request.encode_tlv().expect("encode");
        let decoded = SpecTaskRequest::decode_tlv(&encoded).expect("decode");
        assert_eq!(decoded.task_id, "task-1");
        assert_eq!(decoded.request_hash(), request.request_hash());
    }

    #[test]
    fn proof_before_lock_rejected() {
        let request = SpecTaskRequest {
            task_id: "task-proof".to_string(),
            timestamp: 10,
            capability_token: vec![1, 2, 3],
            delegation_chain: vec![],
            task_type: "cmd:imaging:msi".to_string(),
            target_json: "{}".to_string(),
            parameters_json: "{}".to_string(),
            constraints_json: "{}".to_string(),
            payment_max_sats: 1000,
            timeout_blocks: 5,
            commander_signature: [0u8; 64],
        };
        let settlement = SettlementState::new(&request);
        assert!(!settlement.can_emit_proof());
    }

    #[test]
    fn lock_timeout_rejected() {
        let request = SpecTaskRequest {
            task_id: "task-timeout".to_string(),
            timestamp: 10,
            capability_token: vec![1, 2, 3],
            delegation_chain: vec![],
            task_type: "cmd:imaging:msi".to_string(),
            target_json: "{}".to_string(),
            parameters_json: "{}".to_string(),
            constraints_json: "{}".to_string(),
            payment_max_sats: 1000,
            timeout_blocks: 5,
            commander_signature: [0u8; 64],
        };
        let settlement = SettlementState::new(&request);
        let correlation_id = request.request_hash();
        let lock = SpecPaymentLock {
            task_id: request.task_id.clone(),
            correlation_id,
            payment_hash: derive_payment_hash(correlation_id),
            amount_sats: 1000,
            timeout_blocks: 5,
            timestamp: 12,
        };
        let result = settlement.validate_lock(&lock, 20);
        assert!(result.is_err());
    }

    #[test]
    fn successful_flow_correlation_and_hashes() {
        let request = SpecTaskRequest {
            task_id: "task-flow-2".to_string(),
            timestamp: 10,
            capability_token: vec![1, 2, 3],
            delegation_chain: vec![],
            task_type: "cmd:imaging:msi".to_string(),
            target_json: "{}".to_string(),
            parameters_json: "{}".to_string(),
            constraints_json: "{}".to_string(),
            payment_max_sats: 5000,
            timeout_blocks: 10,
            commander_signature: [0u8; 64],
        };
        let mut settlement = SettlementState::new(&request);
        let correlation_id = request.request_hash();
        let payment_hash = derive_payment_hash(correlation_id);
        let lock = SpecPaymentLock {
            task_id: request.task_id.clone(),
            correlation_id,
            payment_hash,
            amount_sats: 5000,
            timeout_blocks: 10,
            timestamp: 12,
        };
        settlement.validate_lock(&lock, 15).expect("lock valid");
        settlement.mark_locked();
        assert!(settlement.can_emit_accept());
        assert!(settlement.can_emit_proof());
    }

    #[test]
    fn message_envelope_roundtrip() {
        let reject = SpecTaskReject {
            task_id: "task-x".to_string(),
            reason: "invalid".to_string(),
            details: "missing".to_string(),
            timestamp: 42,
        };
        let codec = SpecMessageCodec;
        let encoded = codec
            .encode_message(&SpecMessage::TaskReject(reject.clone()))
            .expect("encode");
        let decoded = codec.decode_message(&encoded).expect("decode");
        match decoded {
            SpecMessage::TaskReject(parsed) => {
                assert_eq!(parsed.task_id, reject.task_id);
                assert_eq!(parsed.reason, reject.reason);
            }
            _ => panic!("unexpected message"),
        }
    }

    #[test]
    fn accept_and_proof_signatures() {
        let executor = keypair();
        let executor_pub = PublicKey::from_keypair(&executor).serialize().to_vec();
        let in_reply_to = sha256(b"request");
        let payment_hash = sha256(b"preimage");
        let mut accept = SpecTaskAccept {
            task_id: "task-accept".to_string(),
            timestamp: 10,
            in_reply_to,
            payment_hash,
            amount_sats: 12000,
            expiry_sec: 3600,
            description: "task-accept".to_string(),
            estimated_duration_sec: 45,
            earliest_start: 12,
            data_volume_mb: 250,
            quality_estimate: 900,
            executor_signature: [0u8; 64],
        };
        accept.executor_signature =
            sign_message_hash(accept.executor_signing_hash(), &executor).expect("sign accept");
        let encoded = accept.encode_tlv().expect("encode accept");
        let decoded = SpecTaskAccept::decode_tlv(&encoded).expect("decode accept");
        let xonly = XOnlyPublicKey::from_keypair(&executor).0;
        assert!(verify_schnorr(
            &decoded.executor_signing_hash(),
            &decoded.executor_signature,
            &xonly
        ));

        let mut proof = SpecProofOfExecution {
            task_id: "task-accept".to_string(),
            task_token_id: [7u8; 16],
            payment_hash,
            output_hash: sha256(b"output"),
            execution_timestamp: 15,
            executor_pubkey: executor_pub,
            executor_signature: [0u8; 64],
        };
        proof.executor_signature =
            sign_message_hash(proof.proof_hash(), &executor).expect("sign proof");
        let encoded = proof.encode_tlv().expect("encode proof");
        let decoded = SpecProofOfExecution::decode_tlv(&encoded).expect("decode proof");
        assert!(verify_schnorr(
            &decoded.proof_hash(),
            &decoded.executor_signature,
            &xonly
        ));
    }

    #[test]
    fn end_to_end_spec_flow() {
        let operator = keypair();
        let commander = keypair();
        let executor = keypair();

        let operator_pub = PublicKey::from_keypair(&operator).serialize().to_vec();
        let executor_pub = PublicKey::from_keypair(&executor).serialize().to_vec();
        let commander_pub = PublicKey::from_keypair(&commander).serialize().to_vec();

        let operator_impl = SpecOperator {
            operator_key: operator,
            operator_pubkey: operator_pub.clone(),
        };
        let issue = TokenIssueRequest {
            subject: commander_pub,
            audience: executor_pub.clone(),
            capability: vec!["cmd:imaging:msi".to_string()],
            issued_at: 1,
            expires_at: 100,
            token_id: None,
        };
        let token = operator_impl.issue_token(&issue).expect("issue token");
        let token_bytes = token.encode_tlv().expect("encode token");

        let mut request = SpecTaskRequest {
            task_id: "task-flow".to_string(),
            timestamp: 10,
            capability_token: token_bytes.clone(),
            delegation_chain: vec![],
            task_type: "cmd:imaging:msi".to_string(),
            target_json: "{}".to_string(),
            parameters_json: "{}".to_string(),
            constraints_json: "{}".to_string(),
            payment_max_sats: 20000,
            timeout_blocks: 144,
            commander_signature: [0u8; 64],
        };
        request.commander_signature =
            sign_message_hash(request.commander_signing_hash(), &commander).expect("sign request");

        let verifier = SpecVerifier {
            operator_pubkey: operator_pub.clone(),
            executor_pubkey: executor_pub.clone(),
        };
        verifier.verify_request(&request, 50).expect("verify request");

        let correlation_id = request.request_hash();
        let payment_hash = derive_payment_hash(correlation_id);
        let mut accept = SpecTaskAccept {
            task_id: request.task_id.clone(),
            timestamp: 11,
            in_reply_to: request.request_hash(),
            payment_hash,
            amount_sats: 15000,
            expiry_sec: 3600,
            description: request.task_id.clone(),
            estimated_duration_sec: 45,
            earliest_start: 15,
            data_volume_mb: 200,
            quality_estimate: 910,
            executor_signature: [0u8; 64],
        };
        accept.executor_signature =
            sign_message_hash(accept.executor_signing_hash(), &executor).expect("sign accept");
        verifier
            .verify_accept(&accept, request.request_hash())
            .expect("verify accept");

        let lock = SpecPaymentLock {
            task_id: request.task_id.clone(),
            correlation_id,
            payment_hash,
            amount_sats: accept.amount_sats,
            timeout_blocks: request.timeout_blocks,
            timestamp: 12,
        };
        let mut settlement = SettlementState::new(&request);
        settlement
            .validate_lock(&lock, 12)
            .expect("validate lock");
        settlement.mark_locked();

        let mut proof = SpecProofOfExecution {
            task_id: request.task_id.clone(),
            task_token_id: token.token_id,
            payment_hash: lock.payment_hash,
            output_hash: sha256(b"output-flow"),
            execution_timestamp: 25,
            executor_pubkey: executor_pub,
            executor_signature: [0u8; 64],
        };
        proof.executor_signature =
            sign_message_hash(proof.proof_hash(), &executor).expect("sign proof");
        verifier.verify_proof(&proof).expect("verify proof");

        let codec = SpecMessageCodec;
        let encoded = codec
            .encode_message(&SpecMessage::TaskRequest(request.clone()))
            .expect("encode message");
        let decoded = codec.decode_message(&encoded).expect("decode message");
        match decoded {
            SpecMessage::TaskRequest(parsed) => {
                assert_eq!(parsed.task_id, request.task_id);
                assert_eq!(parsed.request_hash(), request.request_hash());
            }
            _ => panic!("unexpected message"),
        }
    }
}
