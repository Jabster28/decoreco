use thiserror;

use std::io;

use thiserror::Error;

// #[derive(Error, Debug)]
// pub enum DataStoreError {
//     #[error("data store disconnected")]
//     Disconnect(#[from] io::Error),
//     #[error("the data for key `{0}` is not available")]
//     Redaction(String),
//     #[error("invalid header (expected {expected:?}, found {found:?})")]
//     InvalidHeader {
//         expected: String,
//         found: String,
//     },
//     #[error("unknown data store error")]
//     Unknown,
// }

#[derive(Error, Debug)]
pub enum DecorecoError {
    #[error("file extension {0} is not supported")]
    ExtensionNotSupported(String),
    #[error("unable to run {0}: {1}")]
    CommandFailed(String, String),
    #[error("command failed: {0}")]
    NoSuccessCode(String),
}
