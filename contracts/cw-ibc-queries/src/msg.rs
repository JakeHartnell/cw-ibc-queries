use cosmwasm_std::{Empty, QueryRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Just needs to know the code_id of a reflect contract to spawn sub-accounts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub packet_lifetime: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    IbcQuery {
        channel_id: String,
        // Queries to be executed
        msgs: Vec<QueryRequest<Empty>>,
        // Callback contract address that implements ReceiveIbcResponseMsg
        callback: String,
    },
}
