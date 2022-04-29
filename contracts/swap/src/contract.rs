#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    StdResult, attr, to_binary, Addr, Uint128,
    CosmosMsg, WasmMsg, BankMsg, coin
};

use cw20::{BalanceResponse, Cw20ExecuteMsg};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{STATE, State};
use shared::oracle::PriceResponse;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let initial_state = State {
      owner: info.sender,
      token_address: msg.token_address,
      oracle_address: msg.oracle_address,
    };

    STATE.save(deps.storage, &initial_state)?;

    Ok(Response::new().add_attributes(vec![
        attr("owner", initial_state.owner),
        attr("token_address", initial_state.token_address)
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
      ExecuteMsg::Buy {} => return try_buy(deps, env, info, msg),
      ExecuteMsg::Withdraw { amount } => return try_withdraw(deps, env, info, amount),
      _ => Err(ContractError::NotImplemented {})
    }
    
}

pub fn try_buy(deps: DepsMut, env: Env, info: MessageInfo, _msg: ExecuteMsg) -> Result<Response, ContractError> {
  let price_in_luna = get_price(deps.as_ref())?.price as u128;

  if info.funds.len() == 0 {
      return Err(ContractError::CoinMismatch {})
  }

  let luna_received: Uint128 = info
    .funds
    .iter()
    .find(|c| c.denom == "uluna")
    .map(|c| Uint128::from(c.amount))
    .unwrap_or_else(Uint128::zero);

  let coins_to_be_sent = luna_received.u128() / price_in_luna;
  let coins_in_contract = get_balance_of_cw20(deps.as_ref(), env.contract.address)?.balance.u128();

  if coins_in_contract < coins_to_be_sent { return Err(ContractError::InsufficientCoinsInContract {}) }

  let token_addr = STATE.load(deps.storage)?.token_address;
  let msg_execute = Cw20ExecuteMsg::Transfer {
      recipient: info.sender.to_string(),
      amount: Uint128::from(coins_to_be_sent),
  };

  Ok(Response::new().add_attributes(
    vec![
        ("price", price_in_luna.to_string()),
        ("luna_received", luna_received.to_string()),
        ("coins_sent", coins_to_be_sent.to_string()),
      ]
    ).add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr.to_string(),
        msg: to_binary(&msg_execute)?,
        funds: vec![],
    }))
  )
}

pub fn try_withdraw(deps: DepsMut, env: Env, info: MessageInfo, amount: i32) -> Result<Response, ContractError> {
  let owner = STATE.load(deps.storage)?.owner;
  if info.sender != owner { 
    return Err(ContractError::Unauthorized {})
  }

  let luna_balance = deps.querier.query_balance(env.contract.address, String::from("uluna"))?;
  if luna_balance.amount.u128() < amount as u128 {
    return Err(ContractError::InvalidQuantity{});
  }

  let msg = BankMsg::Send {
    to_address: owner.to_string(),
    amount: vec![coin(amount as u128, String::from("uluna"))],
  };

  Ok(Response::new()
    .add_attributes(vec![
      ("initial_luna_balance", luna_balance.to_string()),
      ("amount", amount.to_string())
    ])
    .add_message(CosmosMsg::Bank(msg)))
}

fn get_price(deps: Deps) -> Result<PriceResponse, ContractError> {
  // let oracle_address = String::from("terra1j3l5strv3hjlujuyvs8s9vgal02gsw6prjwl5j");
  let oracle_address = STATE.load(deps.storage)?.oracle_address;
  let price_response: PriceResponse = deps.querier.query_wasm_smart(
      oracle_address,
      &QueryMsg::QueryPrice {},
  )?;
  Ok(price_response)
}

fn get_balance_of_cw20(deps: Deps, address: Addr) -> Result<BalanceResponse, ContractError> {
  let token_address = STATE.load(deps.storage)?.token_address;
  let balance_response: BalanceResponse = deps.querier.query_wasm_smart(
      token_address,
      &QueryMsg::Balance { address: address }
  )?;
  Ok(balance_response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    // TODO
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    // TODO
    Err(StdError::generic_err("Not implemented"))
}

#[cfg(test)]

mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, Addr};
    use shared::mock_querier::{mock_dependencies};

    const TOKEN: &str = "hyp0000";
    const ORACLE: &str = "oracle000";

    #[test]
    fn proper_initialization() {
      let mut deps = mock_dependencies(&coins(2, "token"));

      let msg = InstantiateMsg { token_address: Addr::unchecked(TOKEN), oracle_address: Addr::unchecked(ORACLE) };
      let info = mock_info("creator", &coins(1000, "earth"));
      let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn try_buy() {
      let mut deps = mock_dependencies(&coins(1000, TOKEN));
      deps.querier.with_oracle_price(10);
      deps.querier.with_token_balances(&[(
        &TOKEN.to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::new(1_000_000_000_000 as u128),
        )],
      )]);

      let msg = InstantiateMsg { token_address: Addr::unchecked(TOKEN), oracle_address: Addr::unchecked("oracle000") };
      let info = mock_info("creator", &coins(1_000_000, "uluna"));
      let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

      let msg = ExecuteMsg::Buy {};
      let info = mock_info("buyer", &coins(1_000, "uluna"));
      let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
      assert_eq!("100", res.attributes[2].value);
    }

    fn try_withdraw() {

    }
}
