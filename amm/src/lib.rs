use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap};
use near_sdk::json_types::{U128};
use near_sdk::{env, Gas, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

mod ft_core;
mod constants;
mod utils;

use crate::ft_core::*;
use crate::utils::*;
use crate::constants::*;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ToyAMM{
  pub owner: AccountId,

  pub token0: AccountId,

  pub token1: AccountId,
    
  pub reserves: LookupMap<AccountId, u128>,

  pub deposit0: LookupMap<AccountId, Balance>,

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
    reserves.insert(&token0.clone(), &0u128);
    reserves.insert(&token1.clone(), &0u128);

    Self {
      owner,
      token0,
      token1,
      reserves,
      deposit0: LookupMap::new(b"d0".to_vec()),
      deposit1: LookupMap::new(b"d1".to_vec()),
    }
  }
  #[inline]
  pub fn assert_liquidity(&self) {
    let (reserve0, reserve1) = self.get_reserves();
    assert!(reserve0 > 0 && reserve1 > 0, "ToyAMM: INSUFFICIENT_LIQUIDITY");
  }

  #[inline]
  pub fn assert_owner(&self) {
    assert!(env::predecessor_account_id() == self.owner, "ToyAMM: ONLY_OWNER_ALLOWED");
  }

  #[inline]
  fn get_deposit(&mut self, token_id: &AccountId) -> &mut LookupMap<AccountId, Balance>{
    if token_id == &self.token0 {
      &mut self.deposit0
    } else if token_id == &self.token1 {
      &mut self.deposit1
    } else {
      panic!("ToyAMM: INVALID_TOKEN_ID");
    }
  }

  fn remove_deposit(&mut self, account_id: &AccountId, token: &AccountId, amount: u128) {
    let deposit = self.get_deposit(&token);
    let balance = deposit.get(account_id).unwrap_or(0);
    assert!(balance >= amount, "ToyAMM: INSUFFICIENT_DEPOSIT");
    deposit.insert(&account_id.clone(), &(balance - amount));
  }
  
  /**
   * ToyAMM requires only owner of the contract
   * currently there is no reward to liquidity providers
   */
  pub fn add_liquidity(&mut self, token0_account: AccountId, amount0_in: U128, token1_account: AccountId, amount1_in: U128) {
    self.assert_owner();

    self.remove_deposit(&env::predecessor_account_id(), &token0_account, amount0_in.into());
    self.remove_deposit(&env::predecessor_account_id(), &token1_account, amount1_in.into());

    let reserve0_new = self.reserves.get(&token0_account).unwrap_or(0) + amount0_in.0;
    let reserve1_new = self.reserves.get(&token1_account).unwrap_or(0) + amount1_in.0;
    self.reserves.insert(&token0_account, &reserve0_new);
    self.reserves.insert(&token1_account, &reserve1_new);
  }

  pub fn get_reserves(&self) -> (u128, u128) {
    (self.reserves.get(&self.token0).unwrap(), self.reserves.get(&self.token1).unwrap())
  }
   
  // swap token
  #[payable]
  pub fn swap_for_token(&mut self, token_in: AccountId, token_out: AccountId, amount_in: U128) -> U128 {
    
    self.assert_liquidity();
    
    let reserve_in = self.reserves.get(&token_in).unwrap();
    let reserve_out = self.reserves.get(&token_out).unwrap();
    
    let amount_out = get_amount_out(amount_in.0, reserve_in, reserve_out);
    println!("amount_out: {}", amount_out);

    let user = env::predecessor_account_id();
    
    self.remove_deposit(&user, &token_in, amount_in.0);
    self.deposit_token(&user, &token_out, amount_out);

    self.withdraw_token(&user, &token_out, amount_out);

    self.update(&token_in, &token_out, &(amount_in.0 + reserve_in), &(reserve_out - amount_out));

    amount_out.into()
  }

  pub fn update(&mut self, token0: &AccountId, token1: &AccountId, reserve0: &u128, reserve1: &u128) {
    self.reserves.insert(token0, reserve0);
    self.reserves.insert(token1, reserve1);
  }

  #[private]
  fn deposit_token(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: u128) {
    let deposit = self.get_deposit(token_id);
    let balance = deposit.get(sender_id).unwrap_or(0);
    deposit.insert(&sender_id, &(balance+amount));
  }

  #[private]
  fn withdraw_token(&mut self, to: &AccountId, token_id: &AccountId, amount: u128) {
    let deposit = self.get_deposit(token_id);
    let balance = deposit.get(to).unwrap_or(0);
    assert!(balance >= amount, "ToyAMM: INSUFFICIENT_DEPOSIT");
    deposit.insert(token_id, &(balance-amount));

    ext_ft_core::ext(token_id.clone())
      .with_static_gas(Gas(5*TGAS.0))
      .ft_transfer(to.clone(), amount.into(), None);

  }
}

#[near_bindgen]
impl FungibleTokenReceiver for ToyAMM {

  #[allow(unused_variables)]
  fn ft_on_transfer(
    &mut self,
    sender_id: AccountId,
    amount: U128,
    msg: String
  ) -> PromiseOrValue<U128> {
    let token_id= env::predecessor_account_id();

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
      assert_eq!(reserve0, 100*NDENOM, "reserve0 should be equal to amount0_in");
      assert_eq!(reserve1, 300*NDENOM, "reserve1 should be equal to amount1_in");
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
      println!("reserve0: {}, reserve1: {}", reserve0, reserve1);
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
      println!("reserve0: {}, reserve1: {}", reserve0, reserve1);
      c.deposit_token(&user, &token0, amount_in.into());
      let amount_out = c.swap_for_token(token0, token1, amount_in.into());
      let (new_reserve0, new_reserve1) = c.get_reserves();
      assert!(amount_out.0 + new_reserve1 == reserve1, "reserve1 does not match");
      assert!(new_reserve0 - amount_in == reserve0, "reserve0 does not match");
  }
}