FROM rust:1.74

WORKDIR /usr/src/common-rpc-proxy
COPY . .

RUN cargo install --path common-rpc-proxy/

CMD ["/usr/local/cargo/bin/common-rpc-proxy"]