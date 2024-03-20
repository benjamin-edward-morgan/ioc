FROM node:alpine3.19 as ui-builder
WORKDIR /usr/ioc-demo-ui
COPY ioc-demo-ui .
RUN npm install
RUN npm run build

FROM rust:alpine3.19 as ioc-builder
WORKDIR /usr/ioc
COPY ioc . 
RUN apk add musl-dev
RUN cargo build -r

FROM alpine:3.19
WORKDIR /ioc
COPY --from=ioc-builder /usr/ioc/target/release/ioc .
COPY --from=ioc-builder /usr/ioc/example-configs .
RUN mkdir assets
COPY --from=ui-builder /usr/ioc-demo-ui/dist assets/

EXPOSE 8080 
ENV RUST_LOG="ioc=debug,info"

CMD ["./ioc", "littlefoot_dev.yml"]
