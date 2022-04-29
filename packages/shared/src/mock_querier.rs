use cosmwasm_std::{testing::{MockApi, MockQuerier, MockStorage}, QueryRequest};
use cosmwasm_std::{Coin, OwnedDeps, Querier, WasmQuery, QuerierResult, from_binary, 
  to_binary, from_slice, SystemError, SystemResult, ContractResult, Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_cosmwasm::{
    TerraQueryWrapper, TerraQuerier
};
use cw20::BalanceResponse as Cw20BalanceResponse;
use std::collections::HashMap;

pub const MOCK_CONTRACT_ADDR: &str = "cosmos2contract";

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub price: u64,
}
// TODO: How can I import directly from oracle.rs file?

pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = MOCK_CONTRACT_ADDR;
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(contract_addr, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    price_querier: PriceQuerier,
    token_querier: TokenQuerier
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryPrice { },
    Balance { address: Addr }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
      let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
          Ok(v) => v,
          Err(e) => {
              return SystemResult::Err(SystemError::InvalidRequest {
                  error: format!("Parsing query request: {}", e),
                  request: bin_request.into(),
              })
          }
      };
      self.handle_query(&request)
    }
}

impl WasmMockQuerier {
  pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
      WasmMockQuerier {
          base,
          price_querier: PriceQuerier::default(),
          token_querier: TokenQuerier::default()
      }
  }

  fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
    match &request {
      QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
        if contract_addr == "oracle000" {
          let response: PriceResponse = PriceResponse { price: self.price_querier.price };
          match from_binary(msg).unwrap() {
            QueryMsg::QueryPrice {} => { 
              return SystemResult::Ok(
                ContractResult::Ok(to_binary(&response).unwrap())
              );
            }
            _ => {
              return SystemResult::Err(SystemError::InvalidRequest {
                error: "Did not match on the query message".to_string(),
                request: msg.as_slice().into()});
            }
          }
        } else {
          match from_binary(msg).unwrap() {
            QueryMsg::Balance { address } => {
              let balance = self.token_querier.get_balance(contract_addr, &address.to_string());
              return SystemResult::Ok(ContractResult::Ok(
                  to_binary(&Cw20BalanceResponse { balance }).unwrap(),
              ));
            }
            _ => { 
              return SystemResult::Err(SystemError::InvalidRequest {
                error: "Did not match on the query message".to_string(),
                request: msg.as_slice().into()});
            }
          }
        }
      }
      _ => self.base.handle_query(request),
    }
  }

  pub fn with_oracle_price(&mut self, price: u64) {
    self.price_querier = PriceQuerier::new(price);
  }

  pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
    self.token_querier = TokenQuerier::new(balances);
  }
}

#[derive(Clone, Default)]
pub struct PriceQuerier {
    price: u64,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl PriceQuerier {
    pub fn new(price: u64) -> Self {
        PriceQuerier {
            price: price,
        }
    }
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }

    pub fn get_balance(&self, token_addr: &str, addr: &str) -> Uint128 {
        let contract_balances = self.balances.get(&token_addr.to_string());
        match contract_balances {
            Some(balances) => *balances.get(&addr.to_string()).unwrap_or(&Uint128::zero()),
            None => Uint128::zero(),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}
