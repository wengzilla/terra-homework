import { Coins, MsgExecuteContract, Coin } from '@terra-money/terra.js';
import { client, wallets } from './library.js';
import { readFile } from 'fs/promises';

let addresses = await readJson('../refs.terrain.json');

let adminWallet = wallets.admin;
let testWallet = wallets.wallet1;
let tokenAddress = addresses.testnet.cw20_token.contractAddresses.default;
let oracleAddress = addresses.testnet.oracle.contractAddresses.default;
let swapAddress = addresses.testnet.swap.contractAddresses.default;

const MAX_CONTRACT_BALANCE = 5 * Math.pow(10, 6);

// Check current price.
let price = await client.wasm.contractQuery(oracleAddress, { query_price: {} });
console.log(`Current price: ${price.price}`);

// Check current balance.
let res = await client.wasm.contractQuery(swapAddress, { query_price: {} });
console.log(`Current price contract is reading: ${res.price}`);

// Check current balance.
res = await client.wasm.contractQuery(tokenAddress, { balance: { address: swapAddress } });
console.log(`Current balance of the swap contract is: ${res.balance}`);

// Top up swap contract.
if (res.balance < MAX_CONTRACT_BALANCE) {
  var msg = new MsgExecuteContract(adminWallet.key.accAddress, tokenAddress, {
    transfer: {
      recipient: swapAddress,
      amount: (MAX_CONTRACT_BALANCE - res.balance).toFixed(0),
    },
  });
  var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
  var result = await client.tx.broadcast(tx);
  console.log(result);
}

// // Check current balance of testWallet.
// res = await client.wasm.contractQuery(tokenAddress, { balance: { address: testWallet.key.accAddress } });
// console.log(`Current balance of the testWallet is: ${res.balance}`);

// // Swap against the contract
// var msg = new MsgExecuteContract(
//   testWallet.key.accAddress,
//   swapAddress,
//   {
//     buy: {},
//   },
//   new Coins({ uluna: 1_000_000 })
// );
// var tx = await testWallet.createAndSignTx({ msgs: [msg] });
// var result = await client.tx.broadcast(tx);
// console.log(result);

// // Check current balance of testWallet.
// res = await client.wasm.contractQuery(tokenAddress, { balance: { address: testWallet.key.accAddress } });
// console.log(`Current balance of the testWallet is: ${res.balance}`);

// Withdraw luna from swap contract
var msg = new MsgExecuteContract(adminWallet.key.accAddress, swapAddress, {
  withdraw: { amount: 3_000_000 },
});
var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
var result = await client.tx.broadcast(tx);
console.log(result);

async function readJson(file) {
  return JSON.parse(await readFile(file));
}
