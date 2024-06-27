# Ord canister

> #### A canister for indexing RUNE UTXOs on bitcoin.
--

## Overview

The Ord canister periodically fetch bitcoin blocks from `btc-rpc-proxy` since `840000` using HTTP-outcall and resolve all transactions to RUNE UTXOs. 

Currently, the Ord canister has been deployed on mainnet: [`o25oi-jaaaa-aaaal-ajj6a-cai`](https://dashboard.internetcomputer.org/canister/o25oi-jaaaa-aaaal-ajj6a-cai) and ready to serve.

## RPC Proxy
Usually, the bitcoin RPC `getblocks` responses are greater than 2M which exceeds the `max_response_bytes` limit of HTTP-outcall, so we have to implement [Range requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Range_requests) in Ord canister then combine the responses into btc raw transactions. 

For the same reason, the ord canister requires the RPC servers to support HTTP Range requests while most RPC providers don't. So, there is a crate `btc-rpc-proxy` behalf the real RPC providers.

When `btc-rpc-proxy` receives a request from Ord canister, if the request header contains a range field, it will split the response body following the range then return to Ord canister. Typically, Ord caister needs to repeat 3 requests on a same block to fully fetch it.

## License
[MIT](LICENSE).
