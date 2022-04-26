use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Price has to be positive.")]
    PriceInstantiationError {}, // Add any other custom errors you like here.
                                // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    #[error("Price cannot be updated.")]
    PriceUpdateError {}, // Add any other custom errors you like here.
}
