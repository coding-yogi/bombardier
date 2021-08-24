FROM rust:1.54.0
COPY ./bombardier ./home/bombardier
WORKDIR ./home/bombardier
ENV RUST_LOG info
ENV REST_SERVER_PORT 9001
ENV WEB_SOCKET_PORT 
EXPOSE 9001
EXPOSE 9000
CMD ["./home/bombardier/bombardier", "hub", "-p", "9000", "-s", "9001"]