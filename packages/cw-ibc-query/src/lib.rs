mod checks;
mod ibc_msg;

use cosmwasm_std::IbcOrder;

pub use crate::checks::{check_order, check_version, SimpleIcaError};
pub use crate::ibc_msg::{
    IbcQueryResponse, PacketMsg, ReceiveIbcResponseMsg, ReceiverExecuteMsg, StdAck,
};

pub const IBC_APP_VERSION: &str = "simple-ica-v1";
pub const APP_ORDER: IbcOrder = IbcOrder::Unordered;
// we use this for tests to ensure it is rejected
pub const BAD_APP_ORDER: IbcOrder = IbcOrder::Ordered;
