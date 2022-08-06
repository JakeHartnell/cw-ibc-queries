use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, IbcPacketAckMsg, MessageInfo, QueryResponse,
    Response, StdResult,
};
use cw_ibc_query::ReceiveIbcResponseMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{IbcQueryResultResponse, LATEST_QUERIES};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    // Do nothing for now
    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;
    match msg {
        ExecuteMsg::ReceiveIbcResponse(ReceiveIbcResponseMsg { msg }) => {
            execute_receive(deps, env, info, msg)
        }
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: IbcPacketAckMsg,
) -> Result<Response, ContractError> {
    // which local channel was this packet send from
    let channel_id = msg.original_packet.src.channel_id.clone();
    // store IBC response for later querying from the smart contract??
    LATEST_QUERIES.save(
        deps.storage,
        &channel_id,
        &IbcQueryResultResponse {
            last_update_time: env.block.time,
            response: msg,
        },
    )?;
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::LatestQueryResult { channel_id } => {
            to_binary(&query_latest_ibc_query_result(deps, channel_id)?)
        }
    }
}

fn query_latest_ibc_query_result(
    deps: Deps,
    channel_id: String,
) -> StdResult<IbcQueryResultResponse> {
    let results = LATEST_QUERIES.load(deps.storage, &channel_id)?;
    Ok(results)
}
