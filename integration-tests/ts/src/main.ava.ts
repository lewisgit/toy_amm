import { Worker, NearAccount, NEAR, BN } from 'near-workspaces';
import anyTest, { TestFn } from 'ava';


const STORAGE_BYTE_COST = '1.5 mN';
const INITIAL_SUPPLY = "1000000000 N";

const AMM_WASM_FILEPATH = "../../amm/release/toy_amm.wasm";
const FT_WASM_FILEPATH = "../../FT/res/fungible_token.wasm";

async function registerUser(ft: NearAccount, user: NearAccount) {
  await user.call(
      ft,
      'storage_deposit',
      { account_id: user },
      // Deposit pulled from ported sim test
      { attachedDeposit: STORAGE_BYTE_COST },
  );
}

async function ft_balance_of(ft: NearAccount, user: NearAccount): Promise<BN> {
  return new BN(await ft.view('ft_balance_of', { account_id: user }));
}

const test = anyTest as TestFn<{
  worker: Worker;
  accounts: Record<string, NearAccount>;
}>;

test.before(async t => {
  const worker = await Worker.init();
  const root = worker.rootAccount;

  console.log("deploying FT0");
  const ft0 = await root.devDeploy(FT_WASM_FILEPATH, {
      initialBalance: NEAR.parse('100 N').toJSON(),
      method: "new",
      args: {
          owner_id: root,
          total_supply: NEAR.parse(INITIAL_SUPPLY).toJSON(),
          metadata: {
                spec: "ft-1.0.0",
                name: "Fungible Token 0",
                symbol: "FT0",
                decimals: 24
            }
      }
  });
  console.log("deploying FT1");
  const ft1 = await root.devDeploy(FT_WASM_FILEPATH, {
      initialBalance: NEAR.parse('100 N').toJSON(),
      method: "new",
      args: {
          owner_id: root,
          total_supply: NEAR.parse(INITIAL_SUPPLY).toJSON(),
          metadata: {
                spec: "ft-1.0.0",
                name: "Fungible Token 1",
                symbol: "FT1",
                decimals: 20
            }
      }
  });

  console.log("Deployed FT0 and FT1.");
  console.log(ft0, ft1);

  console.log("deploying AMM");
  const amm = await root.devDeploy(AMM_WASM_FILEPATH, {
      initialBalance: NEAR.parse('100 N').toJSON(),
  })
  await amm.call(
    amm,
    'new',
    {
      owner: root,
      token0: ft0,
      token1: ft1,
    }
  );
  console.log("Deployed AMM");
  console.log(amm);

  console.log("creating Alice");
  const alice = await root.createSubAccount('ali', {
    initialBalance: NEAR.parse('100 N').toJSON(),
  });

  console.log("registering alice and amm on ft0 ft1");
  await registerUser(ft0, amm);
  await registerUser(ft1, amm);
  await registerUser(ft0, alice);
  await registerUser(ft1, alice);

  console.log("root transfer ft0 to alice");
  await root.call(
    ft0,
    'ft_transfer',
    {
      receiver_id: alice,
      amount: NEAR.parse('1000 N').toJSON(),
    },
    {attachedDeposit: '1'}
    )

  console.log("root transfer ft1 to alice");
  await root.call(
    ft1,
    'ft_transfer',
    {
      receiver_id: alice,
      amount: NEAR.parse('1000 N').toJSON(),
    },
    {attachedDeposit: '1'}
    )
  
  console.log("getting balance of alice");
  let balance_ft0 = await ft_balance_of(ft0, alice);
  let balance_ft1 = await ft_balance_of(ft1, alice);

  console.log("alice ft0 balance: ", balance_ft0);
  console.log("alice ft1 balance: ", balance_ft1);

  t.context.worker = worker;
  t.context.accounts = {root, ft0, ft1, amm, alice};
  
})



test.serial.before(async t => {
    await t.context.accounts.root.call(
      t.context.accounts.ft0,
      'ft_transfer_call',
      {
        receiver_id: t.context.accounts.amm,
        amount: NEAR.parse('300 N').toJSON(),
        msg: "0",
      },
      {
        attachedDeposit: '1',
        gas: '200 Tgas',
      },
    );
    await t.context.accounts.root.call(
      t.context.accounts.ft1,
      'ft_transfer_call',
      {
        receiver_id: t.context.accounts.amm,
        amount: NEAR.parse('700 N').toJSON(),
        msg: "0",
      },
      {
        attachedDeposit: '1',
        gas: '200 Tgas',
      },
    );
    await t.context.accounts.root.call(
      t.context.accounts.amm,
      'add_liquidity',
      {
        token0_account: t.context.accounts.ft0,
        amount0_in: NEAR.parse("300 N").toJSON(),
        token1_account: t.context.accounts.ft1,
        amount1_in: NEAR.parse("700 N").toJSON(),
      },
      {
        gas: '200 Tgas',
      },
    );

    let reserves = await t.context.accounts.amm.view(
      'get_reserves',
    );

    console.log("reserves: ", reserves);
})

test('Swap Token', async t => {
    await t.context.accounts.alice.call(
      t.context.accounts.ft0,
      'ft_transfer_call',
      {
        receiver_id: t.context.accounts.amm,
        amount: NEAR.parse('2 N').toJSON(),
        msg: "0",
      },
      {
        attachedDeposit: '1',
        gas: '200 Tgas',
      },
    );

    await t.context.accounts.alice.call(
      t.context.accounts.amm,
      'swap_for_token',
      {
        token_in: t.context.accounts.ft0,
        token_out: t.context.accounts.ft1,
        amount_in: NEAR.parse("2 N").toJSON(),
      },
      {
        gas: '200 Tgas',
      },
    );

    let balance = await ft_balance_of(t.context.accounts.ft1, t.context.accounts.alice);

    console.log("ft1 balance of alice: ", balance.toString());
})

test.after(async t => {
  await t.context.worker.tearDown().catch(error => {
      console.log('Failed to tear down the worker:', error);
  });
});