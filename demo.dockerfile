FROM rust:bookworm as ioc_builder

WORKDIR /usr/ioc
COPY ioc . 

RUN cargo install --path .

EXPOSE 8080 

ENV RUST_LOG="ioc=debug,info"

CMD ["ioc", "example-configs/pid-second-order.yml"]
