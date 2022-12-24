

use near_sdk::{AccountId, ext_contract, PromiseOrValue};
use near_sdk::json_types::{U128};
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;

#[ext_contract(ext_ft_core)]
pub trait FungibleTokenCore {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);

    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;

    fn ft_total_supply(&self) -> U128;

    fn ft_balance_of(&self, account_id: AccountId) -> U128;

    fn ft_metadata(&self) -> FungibleTokenMetadata;
}