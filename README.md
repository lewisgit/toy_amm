# Toy AMM
This project introduces a demonstration of a simple Automated Market Making (AMM) contract implemented in Rust on the NEAR blockchain platform. The demonstration, ToyAMM, is based on the XYK model, similar to [UniswapV2](https://github.com/Uniswap/v2-core). It is initialized with two Fungible Tokens and allows for users to swap between them using the `swap_for_token` function. For simplicity, only the owner of the ToyAMM Contract is permitted to add liquidity, and extra rewards for liquidity providers are not included.

To use ToyAMM, please follow the instructions provided in this document.

# Prerequisite
## Environment Setup

### Rust
* Version: rustc 1.66.0 (69f9c33d7 2022-12-12)
* Toolchain: wasm32-unknown-unknown
* follow [Near Official Doc](https://docs.near.org/develop/contracts/introduction#rust-and-wasm)

### NodeJS
* install [Node](https://github.com/nvm-sh/nvm) version v16.19.0
* install yarn
```
npm install -g yarn
```
### Deployment Tools
* install [NEAR CLI](https://docs.near.org/tools/near-cli#near-deploy)

### Dependencies
For Ubuntu Users
run the following
```shell
apt install git pkg-config libssl-dev
```
For MacOS Users
```shell
brew install git
```

# Testnet Deployment
## Contract Building
1. clone this project
```shell
git clone https://github.com/lewisgit/toy_amm.git
```
2. build contract
```shell
cd toy_amm/amm
./build.sh
```
compiled contract file will locate in release folder

## Contract Deployment
deploy AMM contract
```shell
near dev-deploy --wasmFile amm/release/toy_amm.wasm --helperUrl https://near-contract-helper.onrender.com
```
Set the AMM acount id `export AMM=dev-1671796804763-30061579106765`. Add create a subaccount as contract owner by running:
```shell
near create-account owner.$AMM --masterAccount $AMM --initialBalance 20
```
Set Owner account id `export OWNER=owner.dev-1671796804763-30061579106765`.
## Fungible Token Deployment
`Make sure every contract you deploy has differenct account id.`
deploy Fungible Token(You can refer to deployment steps in details by directing to [FT](https://github.com/near-examples/FT))
1. use project [FT](https://github.com/near-examples/FT) from [near-example](https://github.com/near-examples). In the root directory of this project, run:
```shell
git clone https://github.com/near-examples/FT.git
```
2. Deploy Fungible Token
```shell
cd FT
near dev-deploy --wasmFile FT/res/fungible_token.wasm --helperUrl https://near-contract-helper.onrender.com
```
3. Deploy Fungible Token Twice and make sure Contracts are deployed to different account. FT0 and FT1 are used here to represent different Fungbile Token Contract.
run `export FT0=dev-1671796926050-32416921021565` and `export FT1=dev-1671797065250-44306645033899` to store the Fungbile Tokens' account id.
4. Initialize FT0 and FT1
run the following commands for FT0 and FT1 respectively.
```shell
near call $FT new '{"owner_id": "'$OWNER'", "total_supply": "1000000000000000", "metadata": { "spec": "ft-1.0.0", "name": "Example Token Name", "symbol": "EXLT", "decimals": $DECIMAL }}' --accountId $FT
```
replace `$DECIMAL` with your preferred one, and use correct `$FT`. Here, `$OWNER` is used as your contract owner account, you can also change it to another account.
Register ToyAMM in FT0 and FT1:
```shell
near call $FT storage_deposit '' --amount 0.00125 --accountId $AMM
```
replace $FT with `$FT0` and `$FT1` respectively.

## AMM Contract Initialization
1. After Fungible Tokens are deployed, initialize ToyAMM Contract by running:
```shell
near call $AMM new '{"owner": "'$OWNER'", "token0": "'$FT0'", "token1": "'$FT1'"}' --accountId $AMM
```

## Add Liquidity
1. Call Contract FT0 and FT1's `tf_transfer_call` to deposit tokens in ToyAMM. Run:
```shell
near call $FT0 ft_transfer_call '{"receiver_id": "'$AMM'", "amount": "30000", "msg": "0"}' --accountId $OWNER --depositYocto "1" --gas "200000000000000"
```
```shell
near call $FT1 ft_transfer_call '{"receiver_id": "'$AMM'", "amount": "70000", "msg": "0"}' --accountId $OWNER --depositYocto "1" --gas "200000000000000"
```
without calling `ft_transfer_call` on both tokens, `add_liquidity` will fail to execute.
2. Call AMM `add_liquidity`:
```shell
near call $AMM add_liquidity '{"token0_account": "'$FT0'","amount0_in": "30000", "token1_account": "'$FT1'", "amount1_in": "70000"}' --accountId $OWNER 
```
after running `add_liquidity`, ToyAMM can be used for token exchange.

## Token Swap
1. create an user Alice:
```shell
near create-account alice.$AMM --masterAccount $AMM --initialBalance 20
```
add save it in env variable `export ALICE=alice.$AMM`.
2. Register Alice for FT0 and FT1:
```shell
near call $FT storage_deposit '' --amount 0.00125 --accountId $ALICE
```
replace $FT with `$FT0` and `$FT1` respectively.
3. Transfer enough FT0 tokens to Alice.
```shell
near call $FT0 ft_transfer '{"receiver_id": "'$ALICE'", "amount": "20", "msg": "0"}' --accountId $OWNER --depositYocto "1" --gas "200000000000000"
```
4. Deposit token FT0 to ToyAMM.
```shell
near call $FT0 ft_transfer_call '{"receiver_id": "'$AMM'", "amount": "20", "msg": "0"}' --accountId $ALICE --depositYocto "1" --gas "200000000000000"
```
5. call ToyAMM `swap_for_token` method:
```shell
near call $AMM swap_for_token '{"token_in": "'$FT0'", "token_out": "'$FT1'", "amount_in": "20"}' --accountId $ALICE 
```
6. Check FT1 balance of Alice
```shell
near view $FT1 ft_balance_of '{"account_id": "'$ALICE'"}'
```
the terminal will print '46', which means by depositing 20\*10^-$DECIMALS_FT0 FT0, Alice exachange 46\*10^-$DECIMALS_FT1 FT1 through ToyAMM.

`All balance are calculated in U128, therefore no need for special treatment of Tokens with arbitrary decimals`.

the example account ids are all deployed on testnet, feel free to explore more about this project as you like.

# Contract Testing
Contract testing covers essential parts of ToyAMM, comprehensive testing is listed in TODO.
## Unit Test
In the root directory of this project, run:
```shell
cd amm
cargo test
```
unit test will test functions of ToyAMM Contract.

## Rust Integration Test
In the root directory of this project, run:
```shell
cd integration-tests/rs
cargo run --example integration-tests
```
## Rust Integration Test
In the root directory of this project, run:
```shell
cd integration-tests/ts
yarn test
```
integration tests will test cross contract call and add_liquidity funtion and swap_for_token function.

# TODO
1. Comprehensive Contract Testing
2. Storage Management
3. Liquidity provider shares
4. AMM factory for convenient AMM contruction

# FAQ
1. near-workspaces-js failure

    check your node version, make sure not version=v16.19.0, version >= v18 will cause bugs in near-workspaces.

2. run `NEAR CLI` timeout
  
    better do deployment on a server that locates in US.

# References
1. Uniswap V2: https://github.com/Uniswap/v2-core
2. Near FT Tutorial: https://github.com/near-examples/ft-tutorial
3. Ref Finance: https://github.com/ref-finance
4. Fungible Token on NEAR: https://docs.near.org/develop/relevant-contracts/ft
  
# LICENSE
[LICENSE](LICENSE)