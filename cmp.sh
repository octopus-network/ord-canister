#!/bin/bash

qc() { 
    dfx canister call ord-indexer get_runes_by_utxo "(\"$1\", $2 : nat32)"
}

R=840104.utxo.2
echo -n '' > $R

while IFS=',' read -r c1 c2 c3 c4 c5
do
    echo -n "$c1,$c2," >> $R
    if output=$(qc $c1 $c2 | grep 'id\|balance' | tr -d "{}:;_" | tr "\n" " " | sed 's#nat32##g' | sed 's#nat64##g' | sed 's#nat##g' | tr -d " idrecordtxblockbalance" | sed 's#==#=#g' | awk -F '=' '{print $3,",",$2,",",$4}' | tr -d " "); then
        echo $output >> $R
    else
        echo "Error" >> $R
    fi
done
