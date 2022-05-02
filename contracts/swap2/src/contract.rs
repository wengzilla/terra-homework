#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StakingMsg,
    StdResult, attr, to_binary, Addr, Uint128,
    CosmosMsg, WasmMsg, BankMsg, coin, Coin,
    DistributionMsg, SubMsg
};

use cw20::{BalanceResponse, Cw20ExecuteMsg};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, QueryTokenAddressResponse};
use crate::state::{STATE, State};
use shared::oracle::PriceResponse;
use terra_cosmwasm::{create_swap_msg, TerraQuerier, ExchangeRatesResponse, TerraMsgWrapper, TerraMsg};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VALIDATOR: &str = "terravaloper1vk20anceu6h9s00d27pjlvslz3avetkvnwmr35";

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
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg {
      ExecuteMsg::Buy {} => try_buy(deps, env, info, msg),
      ExecuteMsg::Withdraw { amount } => try_start_withdraw(deps, env, info, amount),
      ExecuteMsg::WithdrawStep2ConvertRewardsToLuna { amount } => try_convert_rewards(deps, env, info, amount),
      ExecuteMsg::WithdrawStep3SendLuna { amount } => try_send_luna(deps, env, info, amount),
      ExecuteMsg::StartUndelegation { amount } => try_start_undelegation(deps, env, info, amount),
    }
}

pub fn try_buy(deps: DepsMut, env: Env, info: MessageInfo, _msg: ExecuteMsg) -> Result<Response<TerraMsgWrapper>, ContractError> {
  let price_in_luna = get_price(deps.as_ref())?.price as u128;

  if info.funds.len() == 0 {
      return Err(ContractError::CoinMismatch {})
  }

  let luna_received: Uint128 = info
    .funds
    .iter()
    .find(|c| c.denom == String::from("uluna"))
    .map(|c| Uint128::from(c.amount))
    .unwrap_or_else(Uint128::zero);

  let coins_to_be_sent = luna_received.u128() / price_in_luna;
  let coins_in_contract = get_balance_of_cw20(deps.as_ref(), env.contract.address)?.balance.u128();

  if coins_in_contract < coins_to_be_sent { return Err(ContractError::InsufficientCoinsInContract {}) }

  let token_addr = STATE.load(deps.storage)?.token_address;

  let msg_transfer = CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: token_addr.to_string(),
      funds: vec![],
      msg: to_binary(&Cw20ExecuteMsg::Transfer {
          recipient: info.sender.to_string(),
          amount: Uint128::from(coins_to_be_sent),
      })?,
  });

  let msg_delegate = CosmosMsg::Staking(StakingMsg::Delegate {
      validator: String::from(VALIDATOR),
      amount: Coin {
          denom: String::from("uluna"),
          amount: luna_received,
      },
  });

  Ok(Response::new().add_attributes(
    vec![
        ("price", price_in_luna.to_string()),
        ("luna_received", luna_received.to_string()),
        ("coins_sent", coins_to_be_sent.to_string()),
      ]
    ).add_messages(vec![msg_transfer, msg_delegate])
  )
}

pub fn try_start_withdraw(deps: DepsMut, env: Env, info: MessageInfo, amount: u64) -> Result<Response<TerraMsgWrapper>, ContractError> {
  let owner = STATE.load(deps.storage)?.owner;
  if info.sender != owner {
    return Err(ContractError::Unauthorized {});
  }

  // Create a list of submessages that will execute in a series
  let mut submessages: Vec<SubMsg<TerraMsgWrapper>> = vec![];
  // Step 1: Claim rewards
  submessages.push(SubMsg::new(CosmosMsg::Distribution(
    DistributionMsg::WithdrawDelegatorReward {
      validator: String::from(VALIDATOR),
    }
  )));

  // Step 2: Convert rewards
  submessages.push(SubMsg::new(CosmosMsg::Wasm(
    WasmMsg::Execute {
      contract_addr: env.contract.address.to_string(),
      msg: to_binary(&ExecuteMsg::WithdrawStep2ConvertRewardsToLuna { amount })?,
      funds: vec![],
    }
  )));

  // Step 3: Send luna to owner
  submessages.push(SubMsg::new(CosmosMsg::Wasm(
    WasmMsg::Execute {
      contract_addr: env.contract.address.to_string(),
      msg: to_binary(&ExecuteMsg::WithdrawStep3SendLuna { amount })?,
      funds: vec![],
    }
  )));

  Ok(Response::new()
    .add_attribute("method", "try_withdraw")
    .add_submessages(submessages))
}

pub fn try_convert_rewards(deps: DepsMut, env: Env, info: MessageInfo, amount: u64) -> Result<Response<TerraMsgWrapper>, ContractError> {
  // Find all native denoms for which we have a balance.
  let balances = deps.querier.query_all_balances(&env.contract.address)?;
  let denoms: Vec<String> = balances.iter().map(|item| item.denom.clone()).collect();

  let reward_denom = String::from("uluna");
  let exchange_rates = query_exchange_rates(&deps, reward_denom, denoms)?;

  let mut submessages: Vec<SubMsg<TerraMsgWrapper>> = vec![];
  for coin in balances {
      if coin.denom == reward_denom
          || !exchange_rates
              .exchange_rates
              .iter()
              .any(|x| x.quote_denom == coin.denom)
      {
          // ignore luna and any other denom that's not convertible to luna.
          continue;
      }

      // QUESTION: What's the difference between doing a Msg vs. a SubMsg?
      submessages.push(SubMsg::new(create_swap_msg(coin, reward_denom.to_string())));
  }

  Ok(Response::new()
    .add_attribute("method", "try_convert_rewards")
    .add_submessages(submessages))
}

pub fn try_send_luna(deps: DepsMut, env: Env, info: MessageInfo, amount: i32) -> Result<Response<TerraMsgWrapper>, ContractError> {
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
      ("method", "try_send_luna"),
      ("initial_luna_balance", &luna_balance.to_string()),
      ("amount", &amount.to_string())
    ])
    .add_message(CosmosMsg::Bank(msg)))
}

pub fn try_start_undelegation(deps: DepsMut, env: Env, info: MessageInfo, amount: u128) -> Result<Response<TerraMsgWrapper>, ContractError>{
    //read params
    let coin_denom = "uluna";
    let mut messages: Vec<CosmosMsg> = vec![];
    let msg_undelegate: CosmosMsg<TerraMsgWrapper> = CosmosMsg::Staking(StakingMsg::Undelegate {
      validator: String::from(VALIDATOR),
      amount: coin(amount, coin_denom),
    });

    Ok(
      Response::new().add_attributes(vec![
        ("method", "try_start_undelegation"),
        ("amount", &amount.to_string())
      ]).add_messages(vec![msg_undelegate])
    )
}

fn get_price(deps: Deps) -> Result<PriceResponse, ContractError> {
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
  match msg {
    QueryMsg::QueryTokenAddress {} => {
      let token_address_response = QueryTokenAddressResponse { 
        token_address: STATE.load(deps.storage)?.token_address
      };
      return to_binary(&token_address_response);
    },
    QueryMsg::QueryPrice {} => { 
      let price_response: PriceResponse = get_price(deps).unwrap();
      return to_binary(&price_response)
    },
    QueryMsg::Balance { address } => {
      return to_binary(&{ address })
    }
  }
}

pub fn query_exchange_rates(
    deps: &DepsMut,
    base_denom: String,
    quote_denoms: Vec<String>,
) -> StdResult<ExchangeRatesResponse> {
    let querier = TerraQuerier::new(&deps.querier);
    let res: ExchangeRatesResponse = querier.query_exchange_rates(base_denom, quote_denoms)?;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, Addr};
    use testing::mock_querier::{mock_dependencies};

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
    fn try_query() {
      let mut deps = mock_dependencies(&coins(2, "token"));
      deps.querier.with_oracle_price(15);

      let msg = InstantiateMsg { token_address: Addr::unchecked(TOKEN), oracle_address: Addr::unchecked("oracle000") };
      let info = mock_info("creator", &coins(1_000_000, "uluna"));
      let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

      // it worked, let's query the state
      let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap();
      let price_response: PriceResponse = from_binary(&res).unwrap();
      assert_eq!(15, price_response.price);
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
      let info = mock_info("buyer", &coins(1_000, String::from("uluna")));
      let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
      assert_eq!("100", res.attributes[2].value);
    }

    #[test]
    fn try_withdraw() {

    }
}
