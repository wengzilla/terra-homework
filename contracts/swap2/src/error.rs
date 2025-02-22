use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("quantity is invalid")]
    InvalidQuantity,

    #[error("Buy Error")]
    BuyError {},

    #[error("Unknown Error")]
    UnknownError {},

    #[error("Not implemented")]
    NotImplemented {},

    #[error("Only uluna should be passed")]
    CoinMismatch {},

    #[error("Not enough coins remain in contract")]
    InsufficientCoinsInContract {},
}
