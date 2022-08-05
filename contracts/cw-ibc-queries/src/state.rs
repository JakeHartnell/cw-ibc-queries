use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{IbcPacketAckMsg, Timestamp};
use cw_storage_plus::{Item, Map};

pub const PENDING: Item<String> = Item::new("pending");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IbcQueryResultResponse {
    /// last block balance was updated (0 is never)
    pub last_update_time: Timestamp,
    pub response: IbcPacketAckMsg,
}
pub const LATEST_QUERIES: Map<&str, IbcQueryResultResponse> = Map::new("queries");
