use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Trying to perform a invalid operation")]
    OperationNotAllowedError,
}
