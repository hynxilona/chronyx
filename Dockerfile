FROM rust:1.98-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
RUN cargo build --release -p chronyx-node --bin chronyxd \
 && cargo build --release -p chronyx-cli --bin chronyx

FROM debian:bookworm-slim
RUN useradd --create-home --shell /usr/sbin/nologin chronyx
COPY --from=build /src/target/release/chronyxd /usr/local/bin/chronyxd
COPY --from=build /src/target/release/chronyx /usr/local/bin/chronyx
USER chronyx
WORKDIR /home/chronyx

# First boot creates a devnet genesis + identity here. Mount a volume to keep
# the ledger across restarts, and mount a shared genesis.json for a multi-node
# cluster (every node must start from the same genesis hash).
VOLUME ["/data"]
EXPOSE 8899 9000

ENTRYPOINT ["chronyxd", "--data-dir", "/data", "--rpc", "0.0.0.0:8899", "--p2p", "0.0.0.0:9000"]
