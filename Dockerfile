FROM rust:latest
COPY . .
ENV RUST_LOG info
RUN cargo build --release
ENTRYPOINT ["./target/release/bombardier"]