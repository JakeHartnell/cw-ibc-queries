use cosmwasm_std::{Empty, QueryRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Just needs to know the code_id of a reflect contract to spawn sub-accounts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    IbcQuery {
        channel_id: String,
        msgs: Vec<QueryRequest<Empty>>,
        // Optional callback
        callback: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // TODO need a better way of doing this...
    // How many query results do we want to store? Zero? Callbacks only?
    // Get latest query
    LatestQueryResult { channel_id: String },
}
