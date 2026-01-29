mod demo;
mod spec;
mod spec_cbor;
mod tlv;
mod traits;

pub use demo::*;
pub use spec::*;
pub use spec_cbor::*;
pub use tlv::*;
pub use traits::*;

use std::fmt;

#[derive(Debug, Clone)]
pub struct ProtocolError {
    pub reason: String,
}

impl ProtocolError {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug, Clone)]
pub struct VerifyError {
    pub reason: String,
}

impl VerifyError {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for VerifyError {}
