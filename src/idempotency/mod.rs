//! src/idempotency/mod.rs
mod key;
pub use key::IdempotencyKey;

mod persistence;
pub use persistence::{get_saved_response, save_response, NextAction, try_processing};
