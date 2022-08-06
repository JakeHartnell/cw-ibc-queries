use thiserror::Error;

use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};

use cw_ibc_query::SimpleIcaError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseReply(#[from] ParseReplyError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    SimpleIca(#[from] SimpleIcaError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,

    #[error("Invalid reply id")]
    InvalidReplyId,
}
