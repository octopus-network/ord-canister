# Development Guide

This guide helps developers set up their local development environment and run tests for runes-indexer.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Environment Setup](#environment-setup)
  - [Bitcoin Environment](#1-bitcoin-environment)
  - [Project Setup](#2-project-setup)
- [Testing Runes](#testing-runes)

## Prerequisites

Before starting development, ensure you have:
- Git
- Rust and Cargo
- dfx (Internet Computer SDK)

## Environment Setup

### 1. Bitcoin Environment

The runes-indexer needs to fetch block data from the Bitcoin RPC, so we need to start a local Bitcoin node in regtest mode.

#### Setting up a local Bitcoin network
To develop Bitcoin dapps locally, you'll need to set up a local Bitcoin network on your machine. Having your own local Bitcoin network allows you to mine blocks quickly and at-will, which facilitates testing various cases without having to rely on the (slow) Bitcoin testnet or the (even slower) Bitcoin mainnet.

1. Download Bitcoin core v27 (https://bitcoin.org/bin/bitcoin-core-27.0/). It is recommended to use the .tar.gz version for Mac users.
2. Unpack the .tar.gz file:
```bash
tar -xfz bitcoin-27.0-x86_64-apple-darwin.tar.gz
```

3. Create a directory named data inside the unpacked folder:
```bash
cd bitcoin-27.0 && mkdir data
```

4. Create a file called `bitcoin.conf` at the root of the unpacked folder and add the following contents:
```ini
# Enable regtest mode. This is required to setup a private bitcoin network.
regtest=1

# Enable full transaction indexing.
txindex=1

# Dummy credentials that are required by `bitcoin-cli`.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
```

5. Run bitcoind to start the bitcoin client:
```bash
./bin/bitcoind -conf=$(pwd)/bitcoin.conf -datadir=$(pwd)/data --port=18444
```

### 2. Project Setup

1. Clone the repository:
```bash
git clone https://github.com/octopus-network/runes-indexer
cd runes-indexer
```

2. Install and run bitcoin-rpc-proxy (needed because canister HTTPS outcalls are limited to 2MB):
```bash
cargo install --path ./bitcoin-rpc-proxy

# Run the proxy
bitcoin-rpc-proxy --forward http://127.0.0.1:18443 --user ic-btc-integration:QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=

# Test the proxy
curl --http0.9 http://127.0.0.1:8000 --data-binary '{"jsonrpc":"1.0","id":"curltext","method":"getblockhash","params":[0]}' -H 'content-type:text/plain;'
```

3. Deploy and start the indexer:
```bash
# Start dfx
dfx start --clean

# Deploy runes-indexer
dfx deploy runes-indexer --argument '( variant { Init = record { subscribers = vec {}; bitcoin_rpc_url = "http://127.0.0.1:8000"; network = variant { regtest }; } } )'

# Start indexing
dfx canister call runes-indexer start
```

4. Verify deployment:
```bash
# View logs
curl http://bkyz2-fmaaa-aaaaa-qaaaq-cai.raw.localhost:4943/logs | jq

# Check indexed height
dfx canister call runes-indexer get_height
```

## Testing Runes

### Install and Configure Ord

1. Install ord v0.22.1 from [here](https://github.com/ordinals/ord/tree/c8f0a1de5a4287752f3f230bceca33c5d1e7a4d3?tab=readme-ov-file#installation)

2. Start ord indexing service:
```bash
ord --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username ic-btc-integration --bitcoin-rpc-password QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E= --regtest --index-runes server
```

3. Create and fund a wallet:
```bash
# Create wallet
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username ic-btc-integration --bitcoin-rpc-password QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E= wallet create

# Get receive address
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username ic-btc-integration --bitcoin-rpc-password QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E= wallet receive
{
  "addresses": [
    "bcrt1p8c2kthscv98rw4szel3d7gm8lue35x9usga0wntukvn03e2mzckqm0qwkr"
  ]
}

# Fund the wallet (replace with your address)
bitcoin-cli -conf=$(pwd)/bitcoin.conf generatetoaddress 101 bcrt1p8c2kthscv98rw4szel3d7gm8lue35x9usga0wntukvn03e2mzckqm0qwkr

# Check the wallet balance
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username ic-btc-integration --bitcoin-rpc-password QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E= wallet balance
{
  "cardinal": 5000000000,
  "ordinal": 0,
  "runes": {},
  "runic": 0,
  "total": 5000000000
}
```

### Create and Etch a Rune

1. Create a batch file (`/tmp/batch.yaml`):
```yaml
mode: separate-outputs
postage: null
reinscribe: false
etching:
  rune: UNCOMMON•GOODS
  divisibility: 2
  premine: 1000000.00
  supply: 1000000.00
  symbol: $
  turbo: true

inscriptions:
- file: /tmp/batch.yaml
```

2. Etch the rune:
```bash
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username ic-btc-integration --bitcoin-rpc-password QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E= wallet batch --fee-rate 1 --batch /tmp/batch.yaml

Waiting for rune UNCOMMONGOODS commitment f4ec7bf17009a572ea32ce6ad93dfe82777327ce5a89158b3023f2f1d62c710e to mature…
Maturing in...[0s] ⠁ [██████▓                                 ] 1/6
```

3. Mine blocks to complete the etch:
```bash
bitcoin-cli -conf=$(pwd)/bitcoin.conf generatetoaddress 5 bcrt1p8c2kthscv98rw4szel3d7gm8lue35x9usga0wntukvn03e2mzckqm0qwkr

Maturing in...[0s]   [████████████████████████████████████████] 6/6
{
  "commit": "f4ec7bf17009a572ea32ce6ad93dfe82777327ce5a89158b3023f2f1d62c710e",
  "commit_psbt": null,
  "inscriptions": [
    {
      "destination": "bcrt1p8qdu0jaw639er4ejtj9cmfdalczsc64tzt7csfks8e2w8jyp6m2sxcka2y",
      "id": "31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36aei0",
      "location": "31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae:0:0"
    }
  ],
  "parents": [],
  "reveal": "31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae",
  "reveal_broadcast": true,
  "reveal_psbt": null,
  "rune": {
    "destination": "bcrt1pq4x5ru9tduzsqv8lwr84q50uynutmx3g2l8m7eew8cmjjlphuytqvwscx6",
    "location": "31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae:1",
    "rune": "UNCOMMON•GOODS"
  },
  "total_fees": 429
}
```

### Verify Rune Indexing

Check if the rune was properly indexed:
```bash
# Mine a block
bitcoin-cli -conf=$(pwd)/bitcoin.conf generatetoaddress 1 bcrt1p8c2kthscv98rw4szel3d7gm8lue35x9usga0wntukvn03e2mzckqm0qwkr

# Check indexing height
dfx canister call runes-indexer get_height

# Check indexing logs
curl http://bkyz2-fmaaa-aaaaa-qaaaq-cai.raw.localhost:4943/logs | jq | grep etched
    "message": "Rune etched: block_height: 107, txid: 31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae, rune_id: RuneId { block: 107, tx: 1 }",


# Check rune etching
dfx canister call runes-indexer get_etching '("31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae")'
(variant { Ok = opt record { confirmations = 1 : nat32; rune_id = "107:1" } })

# Query rune balance in output
dfx canister call runes-indexer query_runes '(vec { "31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae:1" })'
(
  variant {
    Ok = vec {
      opt vec {
        record {
          id = "107:1";
          confirmations = 1 : nat32;
          divisibility = 2 : nat8;
          amount = 100_000_000 : nat;
          symbol = opt "$";
        };
      };
    }
  },
)

# Get rune entry by ID
dfx canister call runes-indexer get_rune_entry_by_rune_id '("107:1")'
(
  variant {
    Ok = record {
      confirmations = 1 : nat32;
      mints = 0 : nat;
      terms = null;
      etching = "31074ef8783a25fe08a8ea156d8ff102ea0d7e02486226f23c83fdbe623c36ae";
      turbo = true;
      premine = 100_000_000 : nat;
      divisibility = 2 : nat8;
      spaced_rune = "UNCOMMON•GOODS";
      number = 0 : nat64;
      timestamp = 1_737_158_528 : nat64;
      block = 107 : nat64;
      burned = 0 : nat;
      symbol = opt "$";
    }
  },
)
```