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

To run the runes-indexer, you'll need a local Bitcoin node in regtest mode for testing. Follow these steps:

#### A. Set up Bitcoin Core
1. Download Bitcoin Core v27 from https://bitcoin.org/bin/bitcoin-core-27.0/ (recommended: .tar.gz for Mac users)

2. Extract and prepare the environment:
```bash
tar -xfz bitcoin-27.0-x86_64-apple-darwin.tar.gz
cd bitcoin-27.0 && mkdir data
```

3. Generate RPC credentials:
```bash
./rpcauth.py omnity
```
You'll see output like this:
```
String to be appended to bitcoin.conf:
rpcauth=omnity:22c0f458c28d2fec9f12b4f19221c36a$26c2540551e5cc70eb832c63421829da5b577f2d5a1fa8f2f773cbececfee65c
Your password:
0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew
```

4. Create `bitcoin.conf` in the root folder:
```ini
regtest=1
txindex=1
rpcauth=omnity:22c0f458c28d2fec9f12b4f19221c36a$26c2540551e5cc70eb832c63421829da5b577f2d5a1fa8f2f773cbececfee65c
```

#### B. Start the Services

1. Start Bitcoin daemon:
```bash
./bin/bitcoind -conf=$(pwd)/bitcoin.conf -datadir=$(pwd)/data --port=18444
```

2. Set up and run the idempotent-proxy:

We use a modified version of idempotent-proxy that supports [Range requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Range_requests) to handle Bitcoin RPC responses that exceed the 2MB HTTPS outcall limit.
```bash
git clone https://github.com/octopus-network/idempotent-proxy
git checkout runes-indexer

cargo install --path src/idempotent-proxy-server

export USER=omnity:0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew
export URL_LOCAL=http://127.0.0.1:18443
idempotent-proxy-server
```

3. Test the proxy:
```bash
curl http://127.0.0.1:8080/URL_LOCAL \
  -H 'content-type:text/plain;' \
  -H 'idempotency-key: idempotency_key_001' \
  --data-binary '{"jsonrpc":"1.0","id":"curltext","method":"getblockhash","params":[0]}'
```

### 2. Project Setup

1. Clone and deploy the indexer:
```bash
git clone https://github.com/octopus-network/runes-indexer
cd runes-indexer

# Start dfx
dfx start --clean

# Deploy runes-indexer
dfx deploy runes-indexer --argument '( variant { Init = record { subscribers = vec {}; bitcoin_rpc_url = "http://127.0.0.1:8080/URL_LOCAL"; network = variant { regtest }; } } )'

# Start indexing
dfx canister call runes-indexer start
```

2. Verify the deployment:
```bash
# View logs
curl http://bkyz2-fmaaa-aaaaa-qaaaq-cai.raw.localhost:4943/logs | jq

# Check indexed height
dfx canister call runes-indexer get_latest_block
```

## Testing Runes

### 1. Set Up Ord

1. Install ord v0.22.1 from [the official repository](https://github.com/ordinals/ord/tree/c8f0a1de5a4287752f3f230bceca33c5d1e7a4d3?tab=readme-ov-file#installation)

2. Start the ord indexing service:
```bash
ord --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username omnity --bitcoin-rpc-password 0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew --regtest --index-runes server
```

3. Set up and fund a wallet:
```bash
# Create wallet
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username omnity --bitcoin-rpc-password 0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew wallet create
{
  "mnemonic": "dilemma dismiss combine novel online inhale vague length obvious idle have tray",
  "passphrase": ""
}

# Get receive address
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username omnity --bitcoin-rpc-password 0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew wallet receive
{
  "addresses": [
    "bcrt1pu4nzdvaan4n0xvxmcwxatq33zgtx5m43xqwk55x394uhl4ln49us60cpuy"
  ]
}

# Fund the wallet (use your address from the previous command)
bitcoin-cli -rpcuser=omnity -rpcpassword=0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew -conf=$(pwd)/bitcoin.conf generatetoaddress 101 bcrt1pu4nzdvaan4n0xvxmcwxatq33zgtx5m43xqwk55x394uhl4ln49us60cpuy

# Check wallet balance
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username omnity --bitcoin-rpc-password 0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew wallet balance
{
  "cardinal": 5000000000,
  "ordinal": 0,
  "runes": {},
  "runic": 0,
  "total": 5000000000
}
```

### 2. Create and Etch a Rune

1. Create a batch file at `/tmp/batch.yaml`:
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
ord --regtest --bitcoin-rpc-url 127.0.0.1:18443 --bitcoin-rpc-username omnity --bitcoin-rpc-password 0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew wallet batch --fee-rate 1 --batch /tmp/batch.yaml
Waiting for rune UNCOMMONGOODS commitment a32e2549230b2be0c9cb938a4934f2372f369c6fec8808fa74c2f72ca986a3cf to mature…
Maturing in...[0s] ⠁ [██████▓                                 ] 1/6
```

3. Mine blocks to complete the etch:
```bash
bitcoin-cli -rpcuser=omnity -rpcpassword=0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew -conf=$(pwd)/bitcoin.conf generatetoaddress 5 bcrt1pu4nzdvaan4n0xvxmcwxatq33zgtx5m43xqwk55x394uhl4ln49us60cpuy

Maturing in...[0s]   [████████████████████████████████████████] 6/6                                                                                         {
  "commit": "a32e2549230b2be0c9cb938a4934f2372f369c6fec8808fa74c2f72ca986a3cf",
  "commit_psbt": null,
  "inscriptions": [
    {
      "destination": "bcrt1p7z3jmm6mayjv02m4f5cvdj3uq45cmd3svj2hmah5q8fa366ms7dql7m498",
      "id": "e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712i0",
      "location": "e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712:0:0"
    }
  ],
  "parents": [],
  "reveal": "e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712",
  "reveal_broadcast": true,
  "reveal_psbt": null,
  "rune": {
    "destination": "bcrt1p83uyr84gm8u7dlxn3ccm9udyqk8nzv90cu75hqpejkh8rkvmwngq47xdn3",
    "location": "e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712:1",
    "rune": "UNCOMMON•GOODS"
  },
  "total_fees": 429
}
```

### Verify Rune Indexing

Check if the rune was properly indexed:
```bash
# Mine a block
bitcoin-cli -rpcuser=omnity -rpcpassword=0SiSawTIrQqUMTLxQs4vO4lxHaFMJn54020B3weZYew -conf=$(pwd)/bitcoin.conf generatetoaddress 1 bcrt1pu4nzdvaan4n0xvxmcwxatq33zgtx5m43xqwk55x394uhl4ln49us60cpuy

# Check indexing height
dfx canister call runes-indexer get_latest_block

# Check indexing logs
curl http://bkyz2-fmaaa-aaaaa-qaaaq-cai.raw.localhost:4943/logs | jq | grep etched
    "message": "Rune etched: block_height: 107, txid: e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712, rune_id: RuneId { block: 107, tx: 1 }",


# Check rune etching
dfx canister call runes-indexer get_etching '("e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712")'
(opt record { confirmations = 1 : nat32; rune_id = "107:1" })

# Query rune balance in output
dfx canister call runes-indexer get_rune_balances_for_outputs '(vec { "e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712:1" })'
(
  variant {
    Ok = vec {
      opt vec {
        record {
          confirmations = 1 : nat32;
          divisibility = 2 : nat8;
          amount = 100_000_000 : nat;
          rune_id = "107:1";
          symbol = opt "$";
        };
      };
    }
  },
)

# Get rune entry by ID
dfx canister call runes-indexer get_rune_by_id '("107:1")'
(
  opt record {
    confirmations = 1 : nat32;
    mints = 0 : nat;
    terms = null;
    etching = "e1f14042a750b66b07c67380179e1e8e485f03930cc4e7e479ffb51a963b9712";
    turbo = true;
    premine = 100_000_000 : nat;
    divisibility = 2 : nat8;
    spaced_rune = "UNCOMMON•GOODS";
    number = 0 : nat64;
    timestamp = 1_739_023_862 : nat64;
    block = 107 : nat64;
    burned = 0 : nat;
    rune_id = "107:1";
    symbol = opt "$";
  },
)
```