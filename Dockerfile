FROM rust:1.74

WORKDIR /usr/src/btc-rpc-proxy
COPY . .

RUN cargo install --path btc-rpc-proxy/

CMD ["/usr/local/cargo/bin/btc-rpc-proxy", "--forward", "https://solana-mainnet.g.alchemy.com/v2/t25IzpcIjBXhP-LOurqrTWLWmhPuBwsk"]