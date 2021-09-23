FROM rust:1.55-slim-buster as builder
COPY ./ ./home/bombardier
WORKDIR /home/bombardier
RUN apt-get update && apt-get install -y libxml2-dev libssl-dev pkg-config
RUN cargo build --release

FROM debian:buster-slim 
RUN apt-get update && apt-get -y install ca-certificates libxml2-dev libssl-dev
WORKDIR /home
COPY --from=builder ./home/bombardier/target/release/bombardier ./
RUN chmod +x ./bombardier
ENV RUST_LOG debug
EXPOSE 9000
ENTRYPOINT ["./bombardier"]