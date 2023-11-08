FROM rust:1.72.1-slim-bullseye as builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev
COPY Cargo.toml Cargo.lock ./
RUN mkdir src/
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/capcap*

COPY src ./src
COPY .cargo ./.cargo

RUN cargo build --release

COPY ./create-swap.sh /create-swap.sh
COPY ./index.html /index.html

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libssl-dev procps htop
COPY --from=builder /target/release/capcap /usr/local/bin/capcap
COPY --from=builder /create-swap.sh /usr/local/bin/create-swap.sh
COPY --from=builder /index.html /index.html

RUN chmod a+x /usr/local/bin/create-swap.sh

ENTRYPOINT [ "bin/bash", "-c", "/usr/local/bin/create-swap.sh; capcap" ]
