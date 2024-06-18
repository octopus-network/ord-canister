#!/usr/bin/env python3

import json
import requests
import time


# def local():
#     with open("840104", "r") as f:
#         utxos = {}
#         for line in f:
#             v = line.split()
#             tx = json.loads(v[1])
#             for inp in tx["inputs"]:
#                 r = utxos.pop((inp["address"], str(inp["runes"])), "")
#                 if r != "":
#                     print(
#                         f"Spent {inp['runes']} from {inp['address']}, burn => {r[0]}:{r[1]}"
#                     )
#             for vout, out in enumerate(tx["outputs"]):
#                 if len(out["runes"]) > 0 and out["address"] is not None:
#                     utxos[(out["address"], str(out["runes"]))] = (v[0], vout)
#                     print(
#                         f"Added {out['runes']} to {out['address']}, mint => {v[0]}:{vout}"
#                     )
#         for k, v in utxos.items():
#             print(k, "=>", v)


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


def save(filename):
    utxo = {}
    with open(filename, "r") as f:
        for tx in f:
            while True:
                try:
                    explorer(tx.strip(), utxo)
                    break
                except Exception as e:
                    time.sleep(5)
    for k, v in utxo.items():
        r = v[0].split(":")
        print(f"{k[0]},{k[1]},{r[0]},{r[1]},{v[1]}")


if __name__ == "__main__":
    save("840104.tx")
