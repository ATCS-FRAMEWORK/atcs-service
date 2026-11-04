FROM debian:bookworm-slim
WORKDIR /app
COPY target/x86_64-unknown-linux-musl/release/atcs-service /app/
ENTRYPOINT ["/app/atcs-service"]
