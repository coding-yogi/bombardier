FROM rust:1.54-alpine as builder
COPY ./ ./home/bombardier
WORKDIR ./home/bombardier
RUN apk add --update --no-cache g++ gcc libxslt-dev openssl-dev pkgconfig
RUN cargo build --release

FROM alpine:latest  
RUN apk --no-cache add ca-certificates
WORKDIR ./home
COPY --from=builder ./home/bombardier/target/release/bombardier ./
RUN pwd && ls -l && chmod +x ./bombardier
ENV RUST_LOG info
EXPOSE 9000
ENTRYPOINT ["./bombardier"]