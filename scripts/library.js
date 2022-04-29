import fetch from 'isomorphic-fetch';
import { Coins, LCDClient } from '@terra-money/terra.js';
import * as keys from '../keys.terrain.js';
const gasPrices = await fetch('https://bombay-fcd.terra.dev/v1/txs/gas_prices');
const gasPricesJson = await gasPrices.json();

// LCD stands for "Light Client Daemon". I don't really know much about it, but
// this is how you talk to Terra from JS.
const client = new LCDClient({
  URL: 'https://bombay-lcd.terra.dev/', // Use "https://lcd.terra.dev" for prod "http://localhost:1317" for localterra.
  chainID: 'bombay-12', // Use "columbus-5" for production or "localterra".
  gasPrices: { uluna: gasPricesJson['uluna'] }, // Always pay fees in Luna. You can change this to pay fees in other currencies like UST, if you prefer.
  gasAdjustment: '1.5', // Increase gas price slightly so transactions go through smoothly.
  gas: 10000000,
});

import { MnemonicKey } from '@terra-money/terra.js';

const wallets = {
  admin: client.wallet(new MnemonicKey(keys.default.adminKey)),
  wallet1: client.wallet(new MnemonicKey(keys.default.sampleKey2)),
};

export { client, wallets };
