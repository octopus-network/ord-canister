FROM rust:1.83

WORKDIR /usr/src/btc-rpc-proxy
COPY . .

RUN cargo install --path btc-rpc-proxy/

CMD ["/usr/local/cargo/bin/btc-rpc-proxy", "--forward", "https://btc.nownodes.io"]
