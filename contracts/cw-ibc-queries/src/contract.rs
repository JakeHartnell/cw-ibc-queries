use cosmwasm_std::{
    entry_point, to_binary, DepsMut, Empty, Env, IbcMsg, MessageInfo, QueryRequest, Response,
    StdResult,
};

use cw_ibc_query::PacketMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::PACKET_LIFETIME;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    PACKET_LIFETIME.save(deps.storage, &msg.packet_lifetime)?;
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
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    channel_id: String,
    msgs: Vec<QueryRequest<Empty>>,
    callback: String,
) -> Result<Response, ContractError> {
    // validate callback address
    deps.api.addr_validate(&callback)?;

    // construct a packet to send
    let packet = PacketMsg::IbcQuery { msgs, callback };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env
            .block
            .time
            .plus_seconds(PACKET_LIFETIME.load(deps.storage)?)
            .into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_check_remote_balance");
    Ok(res)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::OwnedDeps;

    use cw_ibc_query::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    use crate::ibc::{ibc_channel_connect, ibc_channel_open};

    use super::*;

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            packet_lifetime: 60u64,
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            packet_lifetime: 60u64,
        };
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
}
