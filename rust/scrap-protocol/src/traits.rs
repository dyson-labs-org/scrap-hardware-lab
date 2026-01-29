use crate::{ProtocolError, VerifyError};

pub trait TokenCodec {
    type Token;

    fn encode_token(&self, token: &Self::Token) -> Result<Vec<u8>, ProtocolError>;
    fn decode_token(&self, bytes: &[u8]) -> Result<Self::Token, ProtocolError>;
}

pub trait MessageCodec {
    type Message;

    fn encode_message(&self, msg: &Self::Message) -> Result<Vec<u8>, ProtocolError>;
    fn decode_message(&self, bytes: &[u8]) -> Result<Self::Message, ProtocolError>;
}

pub trait Verifier {
    type Token;
    type Request;
    type Accept;
    type Proof;

    fn verify_token(&self, token: &Self::Token, now: u64) -> Result<(), VerifyError>;
    fn verify_request(&self, request: &Self::Request, now: u64) -> Result<(), VerifyError>;
    fn verify_accept(
        &self,
        accept: &Self::Accept,
        expected_request_hash: [u8; 32],
    ) -> Result<(), VerifyError>;
    fn verify_proof(&self, proof: &Self::Proof) -> Result<(), VerifyError>;
}

pub trait Operator {
    type Token;
    type Accept;
    type Proof;

    fn issue_token(&self, req: &TokenIssueRequest) -> Result<Self::Token, ProtocolError>;
    fn sign_accept(&self, accept: &Self::Accept) -> Result<[u8; 64], ProtocolError>;
    fn sign_proof(&self, proof_hash: [u8; 32]) -> Result<[u8; 64], ProtocolError>;
}

#[derive(Debug, Clone)]
pub struct TokenIssueRequest {
    pub subject: Vec<u8>,
    pub audience: Vec<u8>,
    pub capability: Vec<String>,
    pub issued_at: u32,
    pub expires_at: u32,
    pub token_id: Option<[u8; 16]>,
}
