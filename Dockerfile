FROM rust:1.83

WORKDIR /usr/src/bitcoin-rpc-proxy
COPY . .

RUN cargo install --path bitcoin-rpc-proxy/

CMD ["/usr/local/cargo/bin/bitcoin-rpc-proxy", "--forward", "https://btc.nownodes.io"]
