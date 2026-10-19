FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY solution/Cargo.toml ./Cargo.toml
COPY solution/Cargo.lock ./Cargo.lock
COPY solution/src ./src
RUN cargo test --release && cargo build --release

FROM debian:bookworm-slim
COPY --from=build /app/target/release/filler /usr/local/bin/filler
ENTRYPOINT ["/usr/local/bin/filler"]
