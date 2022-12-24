use near_units::{parse_gas, parse_near};
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

const AMM_WASM_FILEPATH: &str = "../../amm/release/toy_amm.wasm";
const FT_WASM_FILEPATH: &str = "../../FT/res/fungible_token.wasm"; 

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = workspaces::sandbox().await?;

    println!("reading {}", AMM_WASM_FILEPATH);
    let amm_wasm = std::fs::read(AMM_WASM_FILEPATH)?;
    println!("reading {}", FT_WASM_FILEPATH);
    let ft_wasm = std::fs::read(FT_WASM_FILEPATH)?;
    let amm_contract = worker.dev_deploy(&amm_wasm).await?;
    let ft0_contract = worker.dev_deploy(&ft_wasm).await?;
    let ft1_contract = worker.dev_deploy(&ft_wasm).await?;

    // create accounts
    let owner = worker.root_account()?;
    println!("owner: {}", owner.id());
    println!("ft0_contract: {}", ft0_contract.id());
    println!("ft1_contract: {}", ft1_contract.id());
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
            "total_supply": parse_near!("100000000 N").to_string(),
            "metadata": {
                "spec": "ft-1.0.0",
                "name": "Fungible Token 0",
                "symbol": "FT0",
                "decimals": 24
            }
        }))
        .transact()
        .await?
        .into_result()?;

    // initialize ft1
    ft1_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("100000000 N").to_string(),
            "metadata": {
                "spec": "ft-1.0.0",
                "name": "Fungible Token 1",
                "symbol": "FT1",
                "decimals": 20
            }
        }))
        .transact()
        .await?
        .into_result()?;

    // initialize amm contract
    let init_amm = amm_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner": owner.id(),
            "token0": ft0_contract.id(),
            "token1": ft1_contract.id(),
        }))
        .transact()
        .await?;
    println!("init_amm: {:?}", init_amm.logs());
    
    // register ft accounts for AMM contract
    println!("owner call ft0: storage_deposit to amm");
    owner 
        .call(ft0_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
               "account_id": amm_contract.id(),
        }))
        .deposit(parse_near!("0.00125 N"))
        .transact()
        .await?
        .into_result()?;

    println!("owner call ft1: storage_deposit to amm");
    owner 
        .call(ft1_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
               "account_id": amm_contract.id(),
        }))
        .deposit(parse_near!("0.00125 N"))
        .transact()
        .await?
        .into_result()?;

    println!("alice call ft0: storage_deposit");
    alice 
        .call(ft0_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
               "account_id": alice.id(),
        }))
        .deposit(parse_near!("0.00125 N"))
        .transact()
        .await?
        .into_result()?;

    println!("alice call ft1: storage_deposit");
    alice 
        .call(ft1_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": alice.id(),
        }))
        .deposit(parse_near!("0.00125 N"))
        .transact()
        .await?
        .into_result()?;
    
    // prepare some funds for later test
    println!("owner call ft0: ft_transfer to alice");
    owner
        .call(ft0_contract.id(), "ft_transfer")
        .args_json(serde_json::json!({
            "receiver_id": alice.id(),
            "amount": parse_near!("10 N").to_string(),
        }))
        .deposit(1)
        .transact()
        .await?
        .into_result()?;

    println!("owner call ft1: ft_transfer to alice");
    owner
        .call(ft1_contract.id(), "ft_transfer")
        .args_json(serde_json::json!({
            "receiver_id": alice.id(),
            "amount": parse_near!("10 N").to_string(),
        }))
        .deposit(1)
        .transact()
        .await?
        .into_result()?;
    
    let balance: String = worker
    .view(ft1_contract.id(), "ft_balance_of")
    .args_json(serde_json::json!({
        "account_id": alice.id(),
    }))
    .await?.json()?;

    println!("FT1 Balance of alice: {}", balance);

    
    let result =
    owner
        .call(ft0_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": amm_contract.id(),
            "amount": parse_near!("300 N").to_string(),
            "msg": "0",
        }))
        .deposit(1)
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("owner transfer ft0: {:?}", result.logs());

    owner
        .call(ft1_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": amm_contract.id(),
            "amount": parse_near!("700 N").to_string(),
            "msg": "0",
        }))
        .deposit(1)
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?
        .into_result()?;

    let deposit0: String = worker
    .view(amm_contract.id(), "get_user_deposit")
    .args_json(serde_json::json!({
        "token": ft0_contract.id(),
        "user": owner.id(),
    }))
    .await?.json()?;

    println!("Deposit0 of owner: {}", deposit0);
    
    println!("add liquidity");
    owner
        .call(amm_contract.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "token0_account": ft0_contract.id(),
            "amount0_in": parse_near!("300 N").to_string(),
            "token1_account": ft1_contract.id(),
            "amount1_in": parse_near!("700 N").to_string(),
        }))
        .transact()
        .await?
        .into_result()?;

    println!("alice call ft0: ft_transfer_call to amm");
    alice 
        .call(ft0_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": amm_contract.id(),
            "amount": parse_near!("2 N").to_string(),
            "msg": "0",
        }))
        .deposit(1)
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?
        .into_result()?;

    println!("alice call amm: swap_for_token ");

    let swap_result = alice 
        .call(amm_contract.id(), "swap_for_token")
        .args_json(serde_json::json!({
            "token_in": ft0_contract.id(),
            "token_out": ft1_contract.id(),
            "amount_in": parse_near!("2 N").to_string(),
        }))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("swap result: {:?}", swap_result.logs());
        
    let balance_after: String = worker
    .view(ft1_contract.id(), "ft_balance_of")
    .args_json(serde_json::json!({
        "account_id": alice.id(),
    }))
    .await?.json()?;
    println!("After token swap");
    println!("FT1 Balance of alice: {}", balance_after);

    Ok(())

}
