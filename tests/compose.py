#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import sys
import time
import requests


def explorer(txid, utxo):
    response = requests.get(
        f"https://mainnet-indexer-api.runealpha.xyz/transaction/{txid}", timeout=5
    )
    rs = response.json()
    if rs["statusCode"] != 200:
        return
    body = rs["data"]
    for inp in body["vin"]:
        utxo.pop((inp["txid"], inp["vout"]), "")
    for vout, txout in enumerate(body["vout"]):
        if "runeInject" in txout:
            for rune in txout["runeInject"]:
                rune_id = rune["utxo"]["rune_id"]
                balance = rune["utxo"]["amount"]
                utxo[(body["txid"], vout)] = (rune_id, balance)


def check(filename):
    utxo = {}
    with open(filename, "r", encoding="utf-8") as f:
        for tx in f:
            while True:
                try:
                    explorer(tx.strip(), utxo)
                    print(f"fetched {tx}", file=sys.stderr)
                    break
                except Exception as e:
                    print(e, file=sys.stderr)
                    time.sleep(5)
    for k, v in utxo.items():
        r = v[0].split(":")
        print(f"{k[0]},{k[1]},{r[0]},{r[1]},{v[1]}")


if __name__ == "__main__":
    check(sys.argv[1])
