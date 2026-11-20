FROM rust:1.98-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
RUN cargo build --release -p tachyon-node --bin tachyond \
 && cargo build --release -p tachyon-cli --bin tachyon

FROM debian:bookworm-slim
RUN useradd --create-home --shell /usr/sbin/nologin tachyon
COPY --from=build /src/target/release/tachyond /usr/local/bin/tachyond
COPY --from=build /src/target/release/tachyon /usr/local/bin/tachyon
USER tachyon
WORKDIR /home/tachyon

# First boot creates a devnet genesis + identity here. Mount a volume to keep
# the ledger across restarts, and mount a shared genesis.json for a multi-node
# cluster (every node must start from the same genesis hash).
VOLUME ["/data"]
EXPOSE 8899 9000

ENTRYPOINT ["tachyond", "--data-dir", "/data", "--rpc", "0.0.0.0:8899", "--p2p", "0.0.0.0:9000"]
