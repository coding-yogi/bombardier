FROM rust:1.55-alpine as builder
COPY ./ ./home/bombardier
WORKDIR /home/bombardier
RUN apk add --update --no-cache g++ gcc libxslt-dev libgcc openssl-dev make 
RUN RUSTFLAGS="-C target-feature=-crt-static" cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:latest
RUN apk add --update --no-cache libxslt-dev libgcc openssl-dev ca-certificates
WORKDIR /home
COPY --from=builder ./home/bombardier/target/x86_64-unknown-linux-musl/release/bombardier ./
RUN chmod +x ./bombardier
ENV RUST_LOG info
EXPOSE 9000
ENTRYPOINT ["./bombardier"]