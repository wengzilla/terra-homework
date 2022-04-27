import { MsgExecuteContract } from '@terra-money/terra.js';
import { client, wallets } from './library.js';

console.log('Run other scripts...');

let adminWallet = wallets.admin;
let oracleAddress = 'terra1ylhsnjx86aqx6zdtcdu03cykssjdpy6qxvm8m8';

// Check current price.
let price = await client.wasm.contractQuery(oracleAddress, { query_price: {} });
console.log(`Current price: ${price}`);

// Write new price to oracle.
var msg = new MsgExecuteContract(adminWallet.key.accAddress, oracleAddress, {
  update_price: {
    price: 44,
  },
});
var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
var result = await client.tx.broadcast(tx);
console.log(result);

// Check that new price was pushed successfully.
price = await client.wasm.contractQuery(oracleAddress, { query_price: {} });
console.log(`Current price: ${price}`);
