use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum ApiError {
    #[error("internal error")]
    InternalError,
    #[error("wrong credentials")]
    WrongCredentials,
    #[error("missing credentials")]
    MissingCredentials,
    #[error("token creation failed")]
    TokenCreation,
    #[error("invalid token")]
    InvalidToken,
}