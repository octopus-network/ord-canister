# The data types for accessing ord-canister

Currently, the Ord canister has been deployed on mainnet: [`o25oi-jaaaa-aaaal-ajj6a-cai`](https://dashboard.internetcomputer.org/canister/o25oi-jaaaa-aaaal-ajj6a-cai) and ready to serve.

Rust usage:

```
use rune_indexer_interface::*;

let indexer = Principal::from_text("o25oi-jaaaa-aaaal-ajj6a-cai").unwrap();
let (result,): (Result<Vec<RuneBalance>, OrdError>,) = ic_cdk::call(indexer, "get_runes_by_utxo", ("ee8345590d85047c66a0e131153e5202b9bda3990bd07decd9df0a9bb2589348", 0)).await.unwrap();
```

