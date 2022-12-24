use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, ext_ft_metadata, FT_METADATA_SPEC};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap};
use near_sdk::json_types::{U128};
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

mod ft_core;
mod constants;
mod utils;
mod internal;

use crate::utils::*;

/**
 * ToyAMM demostrate a simple AMM of 2 tokens
 * In this contract, only the owner of the contract can add liquidity
 * users can swap token for another token
 * users cannot set the minimum amount of token to receive
 */

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ToyAMM{

  // owner of this contract
  pub owner: AccountId,

  // token0 of this AMM
  pub token0: AccountId,

  // token1 of this AMM
  pub token1: AccountId,

  // token0's metadata
  pub meta0: FungibleTokenMetadata,

  // token1's metadata
  pub meta1: FungibleTokenMetadata,

  // token reserves of token0 and token1
  pub reserves: LookupMap<AccountId, Balance>,

  // store users' deposit on token0
  pub deposit0: LookupMap<AccountId, Balance>,

  // store users' deposit on token1
  pub deposit1: LookupMap<AccountId, Balance>,

}

#[near_bindgen]
impl ToyAMM {
  
  #[init]
  #[private]
  pub fn new(owner: AccountId, token0: AccountId, token1: AccountId) -> Self {
    assert!(env::is_valid_account_id(owner.as_bytes()), "ToyAMM: OWNER_ACCOUNT_ID_INVALID");
    assert!(!env::state_exists(), "ToyAMM: ALREADY_INITIALIZED");

    let mut reserves = LookupMap::new(b"r".to_vec());
    reserves.insert(&token0, &0u128);
    reserves.insert(&token1, &0u128);

    let mut meta0: FungibleTokenMetadata = FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Example NEAR fungible token".to_string(),
                symbol: "EXAMPLE".to_string(),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: 24,
    };
    let mut meta1: FungibleTokenMetadata = meta0.clone();
    ext_ft_metadata::ext(token0.clone())
      .ft_metadata()
      .then(
        Self::ext(env::current_account_id())
          .metadata(&mut meta0)
      );

    ext_ft_metadata::ext(token1.clone())
      .ft_metadata()
      .then(
        Self::ext(env::current_account_id())
          .metadata(&mut meta1)
      );
    log!("meta0: {}, meta1: {}", meta0.decimals, meta1.decimals);

    Self {
      owner,
      token0,
      token1,
      meta0,
      meta1,
      reserves,
      deposit0: LookupMap::new(b"d0".to_vec()),
      deposit1: LookupMap::new(b"d1".to_vec()),
    }
  }

  #[private]
  pub fn metadata(&self, meta: &mut FungibleTokenMetadata, #[callback] token_meta: FungibleTokenMetadata) {
     *meta = token_meta;
  }

  /**
   * get token metadata
   */
  pub fn get_metadata(&self) -> (FungibleTokenMetadata, FungibleTokenMetadata) {
    (self.meta0.clone(), self.meta1.clone())
  }

  /**
   * get user deposit on token
   */ 
  pub fn get_user_deposit(&self, token: AccountId, user: AccountId) -> U128{
    if token == self.token0 {
      U128(self.deposit0.get(&user).unwrap_or(0))
    } else if token == self.token1 {
      U128(self.deposit1.get(&user).unwrap_or(0))
    } else {
      panic!("ToyAMM: INVALID_TOKEN_ID");
    }
  }

  pub fn get_reserves(&self) -> (U128, U128) {
    (self.reserves.get(&self.token0).unwrap().into(), self.reserves.get(&self.token1).unwrap().into())
  }
  
  /**
   * only owner can add liquidity for ToyAMM
   * currently there is no reward to liquidity providers
   */
  pub fn add_liquidity(&mut self, token0_account: AccountId, amount0_in: U128, token1_account: AccountId, amount1_in: U128) {
    self.assert_owner(&env::predecessor_account_id());

    self.remove_deposit(&env::predecessor_account_id(), &token0_account, amount0_in.into());
    self.remove_deposit(&env::predecessor_account_id(), &token1_account, amount1_in.into());

    let reserve0_new = self.reserves.get(&token0_account).unwrap_or(0) + amount0_in.0;
    let reserve1_new = self.reserves.get(&token1_account).unwrap_or(0) + amount1_in.0;
    self.reserves.insert(&token0_account, &reserve0_new);
    self.reserves.insert(&token1_account, &reserve1_new);
  }
   
  /**
   * swap for token
   * before swap, user should have enough deposit greater than amount
   */ 
  #[payable]
  pub fn swap_for_token(&mut self, token_in: AccountId, token_out: AccountId, amount_in: U128) -> U128 {
    
    self.assert_liquidity();
    
    let reserve_in = self.reserves.get(&token_in).unwrap();
    let reserve_out = self.reserves.get(&token_out).unwrap();
    
    let amount_out = get_amount_out(amount_in.0, reserve_in, reserve_out);

    let user = env::predecessor_account_id();
    
    self.remove_deposit(&user, &token_in, amount_in.0);
    self.deposit_token(&user, &token_out, amount_out);

    self.withdraw_token(&user, &token_out, amount_out);

    self.update(&token_in, &token_out, &(amount_in.0 + reserve_in), &(reserve_out - amount_out));

    amount_out.into()
  }

}

#[near_bindgen]
impl FungibleTokenReceiver for ToyAMM {

  /**
   * add user transfer tokens to deposit
   */
  #[allow(unused_variables)]
  fn ft_on_transfer(
    &mut self,
    sender_id: AccountId,
    amount: U128,
    msg: String
  ) -> PromiseOrValue<U128> {
    let token_id= env::predecessor_account_id();

    log!("ToyAMM: transfer receiver. token_id: {} sender_id: {} amount: {}",
        token_id, sender_id, amount.0);

    self.deposit_token(&sender_id, &token_id, amount.0);

    PromiseOrValue::Value(U128(0))
  }

}

#[cfg(test)]
mod tests {
  use super::*;
  use near_sdk::{testing_env};
  use near_sdk::test_utils::{accounts, VMContextBuilder};

  /// 1 NEAR in yocto = 1e24
  pub const NDENOM: u128 = 1_000_000_000_000_000_000_000_000;

  fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(accounts(0))
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id);
    builder
  }


  #[test]
  #[should_panic(expected = "ToyAMM: INSUFFICIENT_DEPOSIT")]
  fn owner_add_liquidity_without_deposit() {
      let token0 = accounts(4);
      let token1 = accounts(5);
      let mut context = get_context(accounts(1));
      testing_env!(context
        .build()
      );
      let mut contract = ToyAMM::new(accounts(1), token0.clone(), token1.clone());
      contract.add_liquidity(
        token0,
        (100 * NDENOM).into(),
        token1,
        (300 * NDENOM).into(),
      );
  }

  #[test]
  fn owner_add_liquidity() {
      let token0 = accounts(4);
      let token1 = accounts(5);
      let user = accounts(2);
      let mut context = get_context(accounts(1));
      testing_env!(context
        .build()
      );
      let mut c= ToyAMM::new(accounts(1), token0.clone(), token1.clone());
      c.deposit_token(&accounts(1), &token0, (100 * NDENOM).into());
      c.deposit_token(&accounts(1), &token1, (300 * NDENOM).into());
      c.add_liquidity(
        token0,
        (100 * NDENOM).into(),
        token1,
        (300 * NDENOM).into(),
      );
      let (reserve0, reserve1) = c.get_reserves();
      assert_eq!(reserve0.0, 100*NDENOM, "reserve0 should be equal to amount0_in");
      assert_eq!(reserve1.0, 300*NDENOM, "reserve1 should be equal to amount1_in");
  }

  #[test]
  #[should_panic(expected = "ToyAMM: INSUFFICIENT_LIQUIDITY")]
  fn swap_for_token_no_liquidity() {
      let token0 = accounts(4);
      let token1 = accounts(5);
      let user = accounts(2);
      let mut context = get_context(accounts(1));
      testing_env!(context
        .build()
      );
      let mut c= ToyAMM::new(accounts(1), token0.clone(), token1.clone());
      testing_env!(context
        .predecessor_account_id(user.clone())
        .build()
      );
      let amount_in = 1*NDENOM;
      let (reserve0, reserve1) = c.get_reserves();
      println!("reserve0: {}, reserve1: {}", reserve0.0, reserve1.0);
      c.deposit_token(&user, &token0, amount_in.into());
      let amount_out = c.swap_for_token(token0, token1, amount_in.into());
  }

  #[test]
  fn swap_for_token() {
      let token0 = accounts(4);
      let token1 = accounts(5);
      let user = accounts(2);
      let owner = accounts(1);
      let mut context = get_context(owner.clone());
      testing_env!(context
        .build()
      );
      let mut c= ToyAMM::new(owner.clone(), token0.clone(), token1.clone());
      testing_env!(context
        .predecessor_account_id(owner.clone())
        .build()
      );

      let add_amount0 = (100*NDENOM);
      let add_amount1 = (300*NDENOM);

      c.deposit_token(&owner, &token0, add_amount0.into());
      c.deposit_token(&owner, &token1, add_amount1.into());
      
      c.add_liquidity(token0.clone(), add_amount0.into(), token1.clone(), add_amount1.into());

      testing_env!(context
        .predecessor_account_id(user.clone())
        .build()
      );

      let amount_in = 1*NDENOM;
      let (reserve0, reserve1) = c.get_reserves();
      println!("reserve0: {}, reserve1: {}", reserve0.0, reserve1.0);
      c.deposit_token(&user, &token0, amount_in.into());
      let amount_out = c.swap_for_token(token0, token1, amount_in.into());
      let (new_reserve0, new_reserve1) = c.get_reserves();
      assert!(amount_out.0 + new_reserve1.0 == reserve1.0, "reserve1 does not match");
      assert!(new_reserve0.0 - amount_in == reserve0.0, "reserve0 does not match");
  }
}
