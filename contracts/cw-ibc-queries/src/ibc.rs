use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, Deps, DepsMut, Empty, Env, Event,
    Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, QueryRequest, StdResult, SystemResult, WasmMsg,
};
use cw_ibc_query::{
    check_order, check_version, IbcQueryResponse, PacketMsg, ReceiveIbcResponseMsg, StdAck,
    IBC_APP_VERSION,
};

use crate::error::ContractError;
use crate::state::{IbcQueryResultResponse, LATEST_QUERIES, PENDING};

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    let channel = msg.channel();

    check_order(&channel.order)?;
    // In ibcv3 we don't check the version string passed in the message
    // and only check the counterparty version.
    if let Some(counter_version) = msg.counterparty_version() {
        check_version(counter_version)?;
    }

    // We return the version we need (which could be different than the counterparty version)
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_APP_VERSION.to_string(),
    }))
}

#[entry_point]
/// once it's established, we create the reflect contract
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let chan_id = &channel.endpoint.channel_id;

    // store the channel id for the reply handler
    PENDING.save(deps.storage, chan_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", chan_id)
        .add_event(Event::new("ibc").add_attribute("channel", "connect")))
}

#[entry_point]
/// On closed channel, we take all tokens from reflect contract to this contract.
/// We also delete the channel entry from accounts.
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    // get contract address and remove lookup
    let channel_id = channel.endpoint.channel_id.as_str();

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let msg: PacketMsg = from_slice(&msg.packet.data)?;
    match msg {
        PacketMsg::IbcQuery { msgs, .. } => receive_query(deps.as_ref(), msgs),
    }
}

// Processes IBC query
pub fn receive_query(
    deps: Deps,
    msgs: Vec<QueryRequest<Empty>>,
) -> Result<IbcReceiveResponse, ContractError> {
    let mut results: Vec<Binary> = vec![];

    for query in msgs {
        let res = match deps.querier.raw_query(&to_binary(&query)?) {
            SystemResult::Ok(res) => res,
            SystemResult::Err(err) => cosmwasm_std::ContractResult::Err(err.to_string()),
        };
        results.push(to_binary(&res)?);
    }
    let response = IbcQueryResponse { results };

    let acknowledgement = StdAck::success(&response);
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_ibc_query"))
}

#[entry_point]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // which local channel was this packet send from
    let caller = msg.original_packet.src.channel_id.clone();
    // we need to parse the ack based on our request
    let original_packet: PacketMsg = from_slice(&msg.original_packet.data)?;

    match original_packet {
        PacketMsg::IbcQuery { callback, .. } => acknowledge_query(deps, env, caller, callback, msg),
    }
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}

fn acknowledge_query(
    deps: DepsMut,
    env: Env,
    channel_id: String,
    callback: Option<String>,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // store IBC response for later querying from the smart contract??
    LATEST_QUERIES.save(
        deps.storage,
        &channel_id,
        &IbcQueryResultResponse {
            last_update_time: env.block.time,
            response: msg.clone(),
        },
    )?;
    match callback {
        Some(callback) => {
            // Send IBC packet ack message to another contract
            let msg = WasmMsg::Execute {
                contract_addr: callback.clone(),
                msg: to_binary(&ReceiveIbcResponseMsg { msg })?,
                funds: vec![],
            };
            Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_ibc_query")
                .add_attribute("callback_address", callback)
                .add_message(msg))
        }
        None => Ok(IbcBasicResponse::new().add_attribute("action", "acknowledge_ibc_query")),
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_ibc_packet_ack},
        BankQuery, IbcAcknowledgement,
    };

    use crate::msg::InstantiateMsg;

    use super::*;

    const CHANNEL: &str = "channel-42";

    #[test]
    fn try_receive_query() {
        let deps = mock_dependencies();

        let res = receive_query(
            deps.as_ref(),
            vec![QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: String::from("test"),
            })],
        );
        assert!(res.is_ok());
    }

    #[test]
    fn try_acknowledge_query() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let ack = IbcAcknowledgement::new([]);
        let ibc_res = mock_ibc_packet_ack(CHANNEL, &InstantiateMsg {}, ack).unwrap();
        let res = acknowledge_query(deps.as_mut(), env, CHANNEL.to_string(), None, ibc_res);
        assert!(res.is_ok());
    }
}
