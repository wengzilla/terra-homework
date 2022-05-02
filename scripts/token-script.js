import { MsgExecuteContract } from '@terra-money/terra.js';
import { client, wallets } from './library.js';

console.log('Run other scripts...');

let adminWallet = wallets.admin;
let tokenAddress = 'terra185hhwh456gy603gt2shmme935u0z2vd3x0pfs8';

let response = await client.wasm.contractQuery(tokenAddress, { minter: {} });
console.log(`Minter wallet: ${response.minter}`);

// var msg = new MsgExecuteContract(adminWallet.key.accAddress, tokenAddress, {
//   burn: {
//     amount: (5_000 * Math.pow(10, 6)).toFixed(0),
//   },
// });
// var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
// var result = await client.tx.broadcast(tx);
// console.log(result);

// var msg = new MsgExecuteContract(adminWallet.key.accAddress, tokenAddress, {
//   transfer: {
//     recipient: wallets.wallet1.key.accAddress,
//     amount: (5_000 * Math.pow(10, 6)).toFixed(0),
//   },
// });
// var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
// var result = await client.tx.broadcast(tx);
// console.log(result);

var msg = new MsgExecuteContract(adminWallet.key.accAddress, tokenAddress, {
  mint: {
    recipient: adminWallet.key.accAddress,
    amount: (1_000 * Math.pow(10, 6)).toFixed(0),
  },
});

var tx = await adminWallet.createAndSignTx({ msgs: [msg] });
var result = await client.tx.broadcast(tx);
console.log(result);

response = await client.wasm.contractQuery(tokenAddress, { balance: { address: wallets.wallet1.key.accAddress } });
console.log(response);
