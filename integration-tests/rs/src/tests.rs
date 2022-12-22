use near_units::{parse_gas, parse_near};
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

const AMM_WASM_FILEPATH: &str = "../../amm/releasedefi.wasm";
const FT_WASM_FILEPATH: &str = "../../FT/res/fungible_token.wasm";


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = workspaces::sandbox().await?;

    let amm_wasm = std::fs::read(AMM_WASM_FILEPATH)?;
    let ft_wasm = std::fs::read(FT_WASM_FILEPATH)?;
    let amm_contract = worker.dev_deploy(&amm_wasm).await?;
    let ft0_contract = worker.dev_deploy(&ft_wasm).await?;
    let ft1_contract = worker.dev_deploy(&ft_wasm).await?;

    // create accounts
    let owner = worker.root_account()?;
    let alice = owner
        .create_subaccount("alice")
        .initial_balance(parse_near!("20 N"))
        .transact()
        .await?
        .into_result()?;
    
    // initialize ft0
    ft0_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("").to_string(),
            "metadata": {
                "spec": "ft-1.0.0",
                "name": "Fungible Token 0",
                "symbol": "FT0",
                "decimals": 24
            }
        }))
        .transact()
        .await?;

    // initialize ft1
    ft1_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("").to_string(),
            "metadata": {
                "spec": "ft-1.0.0",
                "name": "Fungible Token 1",
                "symbol": "FT1",
                "decimals": 20
            }
        }))
        .transact()
        .await?;

    // initialize amm contract

    Ok(())

}