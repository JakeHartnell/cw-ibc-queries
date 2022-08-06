use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Empty, Env, IbcMsg, MessageInfo, QueryRequest,
    QueryResponse, Response, StdResult,
};
use cw_ibc_query::PacketMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{IbcQueryResultResponse, LATEST_QUERIES};

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

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
        ExecuteMsg::IbcQuery {
            channel_id,
            msgs,
            callback,
        } => execute_ibc_query(deps, env, info, channel_id, msgs, callback),
    }
}

pub fn execute_ibc_query(
    _deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    channel_id: String,
    msgs: Vec<QueryRequest<Empty>>,
    callback: Option<String>,
) -> Result<Response, ContractError> {
    // construct a packet to send
    let packet = PacketMsg::IbcQuery { msgs, callback };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_check_remote_balance");
    Ok(res)
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
    Ok(results.into())
}

#[cfg(test)]
mod tests {
    use crate::ibc::{ibc_channel_connect, ibc_channel_open};

    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_close_init, mock_ibc_channel_connect_ack,
        mock_ibc_channel_open_init, mock_ibc_channel_open_try, mock_ibc_packet_recv, mock_info,
        mock_wasmd_attr, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::OwnedDeps;
    use cw_ibc_query::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len())
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", APP_ORDER, "reflect");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }

    #[test]
    fn proper_handshake_flow() {
        let mut deps = setup();
        let channel_id = "channel-1234";

        // first we try to open with a valid handshake
        let handshake_open = mock_ibc_channel_open_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.as_mut(), mock_env(), handshake_connect).unwrap();
        assert_eq!(0, res.messages.len());
    }

    //// TODO
    // #[test]
    // fn handle_ibc_query_packet() {
    //     let mut deps = setup();

    //     let channel_id = "channel-123";
    //     let account = "acct-123";

    // // receive a packet for an unregistered channel returns app-level error (not Result::Err)
    // let msgs_to_dispatch = vec![BankMsg::Send {
    //     to_address: "my-friend".into(),
    //     amount: coins(123456789, "uatom"),
    // }
    // .into()];
    // let ibc_msg = PacketMsg::Dispatch {
    //     msgs: msgs_to_dispatch.clone(),
    // };
    // let msg = mock_ibc_packet_recv(channel_id, &ibc_msg).unwrap();
    // // this returns an error
    // ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap_err();

    // // register the channel
    // connect(deps.as_mut(), channel_id, account);

    // // receive a packet for an unregistered channel returns app-level error (not Result::Err)
    // let msg = mock_ibc_packet_recv(channel_id, &ibc_msg).unwrap();
    // let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

    // // assert app-level success
    // let ack: StdAck = from_slice(&res.acknowledgement).unwrap();
    // ack.unwrap();

    // // and we dispatch the BankMsg via submessage
    // assert_eq!(1, res.messages.len());
    // assert_eq!(RECEIVE_DISPATCH_ID, res.messages[0].id);

    // // parse the output, ensuring it matches
    // if let CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr,
    //     msg,
    //     funds,
    // }) = &res.messages[0].msg
    // {
    //     assert_eq!(account, contract_addr.as_str());
    //     assert_eq!(0, funds.len());
    //     // parse the message - should callback with proper channel_id
    //     let rmsg: cw1_whitelist::msg::ExecuteMsg = from_slice(msg).unwrap();
    //     assert_eq!(
    //         rmsg,
    //         cw1_whitelist::msg::ExecuteMsg::Execute {
    //             msgs: msgs_to_dispatch
    //         }
    //     );
    // } else {
    //     panic!("invalid return message: {:?}", res.messages[0]);
    // }

    // // invalid packet format on registered channel also returns error
    // let bad_data = InstantiateMsg { cw1_code_id: 12345 };
    // let msg = mock_ibc_packet_recv(channel_id, &bad_data).unwrap();
    // ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap_err();
    // }
}
