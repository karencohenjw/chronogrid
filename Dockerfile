FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --create-home chronogrid
COPY --from=builder /src/target/release/chronogrid /usr/local/bin/chronogrid
USER 10001:10001
ENTRYPOINT ["chronogrid"]
