#docker build -t nfrankel/rust-dalle:0.1.0 .
FROM --platform=linux/amd64 rust:1-slim-bullseye as build

RUN apt-get update && apt-get -y install pkg-config libssl-dev

COPY Cargo.toml .
COPY Cargo.lock .
COPY src src

RUN cargo build --release

FROM --platform=linux/amd64 debian:bullseye-slim

RUN apt-get update && apt-get -y install ca-certificates

COPY --from=build /target/release/rust-dalle .

ADD templates templates

EXPOSE 3000

ENTRYPOINT ["./rust-dalle" ]