#![forbid(unsafe_code)]

//! Compatibility facade. The canonical capability registry lives in
//! `metadata`; this module keeps the v1 public Rust entry point stable.

pub use crate::metadata::{
    CapabilityManifest as Capabilities, OperationMetadata, ProviderMetadata, ReasonMetadata,
    ResourceLimits, TransactionCapabilities,
};

pub fn current() -> Capabilities {
    crate::metadata::capabilities()
}
