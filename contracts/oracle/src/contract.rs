#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary,
};
use cw2::set_contract_version;

use crate::error::{ContractError};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{STATE, State};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: instantiate contract
    let initial_state = State {
      owner: info.sender,
      price: msg.price
    };

    STATE.save(deps.storage, &initial_state)?;

    Ok(Response::new()
      .add_attribute("owner", initial_state.owner)
      .add_attribute("price", initial_state.price.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    //TODO: execute try_update_price
    match msg {
      ExecuteMsg::UpdatePrice { price } => { 
        let update_fn = |state: State| -> Result<State, ContractError> {
          Ok(State { price, owner: state.owner })
        };

        STATE.update(deps.storage, update_fn)?;
      }
    };

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO
    match msg {
      QueryMsg::QueryPrice {} => { 
        let res = STATE.load(deps.storage)?.price;
        to_binary(&res)
      }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { price: 17 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap();
        let price: u64 = from_binary(&res).unwrap();
        assert_eq!(17, price);
    }

    #[test]
    fn update_price() {
      let mut deps = mock_dependencies(&coins(2, "token"));

      let msg = InstantiateMsg { price: 17 };
      let info = mock_info("creator", &coins(1000, "earth"));
      let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
      
      // it worked, let's query the state
      let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap();
      let price: u64 = from_binary(&res).unwrap();
      assert_eq!(17, price);

      let info = mock_info("oracle", &coins(2, "token"));
      let msg = ExecuteMsg::UpdatePrice { price: 18 };
      let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

      // it worked, let's query the state
      let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap();
      let price: u64 = from_binary(&res).unwrap();
      assert_eq!(18, price);
    }
}
