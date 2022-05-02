import { MsgExecuteContract } from '@terra-money/terra.js';
import { client, wallets } from './library.js';
import { readFile } from 'fs/promises';

let addresses = await readJson('../refs.terrain.json');

console.log('Run other scripts...');

let adminWallet = wallets.admin;
let oracleAddress = addresses.testnet.oracle.contractAddresses.default;

// Check current price.
let res = await client.wasm.contractQuery(oracleAddress, { query_price: {} });
console.log(`Current price: ${res.price}`);

// Write new price to oracle.
var msg = new MsgExecuteContract(adminWallet.key.accAddress, oracleAddress, {
  update_price: {
    price: 10,
  },
});
var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
var result = await client.tx.broadcast(tx);
console.log(result);

// Check that new price was pushed successfully.
res = await client.wasm.contractQuery(oracleAddress, { query_price: {} });
console.log(`Current price: ${res.price}`);

async function readJson(file) {
  return JSON.parse(await readFile(file));
}
