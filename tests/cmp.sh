#!/bin/bash

qc() { 
    dfx canister call ord-canister get_runes_by_utxo --ic "(\"$1\", $2 : nat32)"
}

while IFS=',' read -r c1 c2 c3 c4 c5
do
    echo -n "$c1,$c2,"
    if output=$(qc $c1 $c2 | grep 'id\|balance' | tr -d "{}:;_" | tr "\n" " " | sed 's#nat32##g' | sed 's#nat64##g' | sed 's#nat##g' | tr -d " idrecordtxblockbalance" | sed 's#==#=#g' | awk -F '=' '{print $3,",",$2,",",$4}' | tr -d " "); then
        echo $output
    else
        echo "Error"
    fi
done
