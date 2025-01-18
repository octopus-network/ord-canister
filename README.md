# Ord canister

> #### A canister for indexing RUNE UTXOs on bitcoin.

## Overview

The Ord canister periodically fetch bitcoin blocks from `bitcoin-rpc-proxy` since `840000` using HTTP-outcall and resolve all transactions to RUNE UTXOs. The main purpose of this canister is providing an online decentralized RUNE indexer for querying all etched RUNE assets given a UTXO.

Currently, the Ord canister has been deployed on mainnet: [`o25oi-jaaaa-aaaal-ajj6a-cai`](https://dashboard.internetcomputer.org/canister/o25oi-jaaaa-aaaal-ajj6a-cai) and ready to serve.

Rust usage:

```
use runes_indexer_interface::*;

let indexer = Principal::from_text("o25oi-jaaaa-aaaal-ajj6a-cai").unwrap();
let (result,): (Result<Vec<RuneBalance>, OrdError>,) = ic_cdk::call(indexer, "get_runes_by_utxo", ("ee8345590d85047c66a0e131153e5202b9bda3990bd07decd9df0a9bb2589348", 0)).await.unwrap();
```


## RPC Proxy
Usually, the bitcoin RPC `getblocks` responses are greater than 2M which exceeds the `max_response_bytes` limit of HTTP-outcall, so we have to implement [Range requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Range_requests) in Ord canister then combine the responses into btc raw transactions. 

For the same reason, the ord canister requires the RPC servers to support HTTP Range requests while most RPC providers don't. So, there is a crate `bitcoin-rpc-proxy` behalf the real RPC providers.

When `bitcoin-rpc-proxy` receives a request from Ord canister, if the request header contains a range field, it will split the response body following the range then return to Ord canister. Typically, Ord caister needs to repeat 3 requests on a same block to fully fetch it.

## License
[MIT](LICENSE).
