FROM rust:1.74

WORKDIR /usr/src/btc-rpc-proxy
COPY . .

RUN cargo install --path btc-rpc-proxy/

CMD ["btc-rpc-proxy --forward https://go.getblock.io"]
