use near_sdk::{log, Gas, AccountId, Balance};
use near_sdk::collections::{LookupMap};

use crate::ToyAMM;
use crate::ft_core::*;
use crate::constants::*;

impl ToyAMM {

  #[inline]
  pub(crate) fn assert_liquidity(&self) {
    let (reserve0, reserve1) = self.get_reserves();
    assert!(reserve0.0 > 0 && reserve1.0 > 0, "ToyAMM: INSUFFICIENT_LIQUIDITY");
  }

  #[inline]
  pub(crate) fn assert_owner(&self, user: &AccountId) {
    assert!(user == &self.owner, "ToyAMM: ONLY_OWNER_ALLOWED");
  }

  #[inline]
  pub(crate) fn get_deposit(&mut self, token_id: &AccountId) -> &mut LookupMap<AccountId, Balance>{
    if token_id == &self.token0 {
      &mut self.deposit0
    } else if token_id == &self.token1 {
      &mut self.deposit1
    } else {
      panic!("ToyAMM: INVALID_TOKEN_ID");
    }
  }

  #[inline]
  pub(crate) fn remove_deposit(&mut self, account_id: &AccountId, token: &AccountId, amount: u128) {
    let deposit = self.get_deposit(&token);
    let balance = deposit.get(account_id).unwrap_or(0);
    assert!(balance >= amount, "ToyAMM: INSUFFICIENT_DEPOSIT");
    deposit.insert(&account_id, &(balance - amount));
  }

  pub(crate) fn update(&mut self, token0: &AccountId, token1: &AccountId, reserve0: &u128, reserve1: &u128) {
    self.reserves.insert(token0, reserve0);
    self.reserves.insert(token1, reserve1);
  }

  pub(crate) fn deposit_token(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: u128) {
    let deposit = self.get_deposit(token_id);
    let balance = deposit.get(sender_id).unwrap_or(0);
    deposit.insert(&sender_id, &(balance+amount));
  }

  pub(crate) fn withdraw_token(&mut self, to: &AccountId, token_id: &AccountId, amount: u128) {
    let deposit = self.get_deposit(token_id);
    let balance = deposit.get(to).unwrap_or(0);
    assert!(balance >= amount, "ToyAMM: INSUFFICIENT_DEPOSIT");
    deposit.insert(token_id, &(balance-amount));

    log!("withdraw to: {}, token: {}, amount: {}", to, token_id, amount);

    ext_ft_core::ext(token_id.clone())
      .with_attached_deposit(1)
      .with_static_gas(Gas(5*TGAS.0))
      .ft_transfer(to.clone(), amount.into(), None);

  }

}