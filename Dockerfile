# Create the build container to compile the hello world program
FROM rust:1.48 as builder
RUN USER=root cargo new --bin kafka-healthcheck
WORKDIR ./kafka-healthcheck
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD src/ ./src
RUN rm ./target/release/deps/kafka_exporter*
RUN cargo build --release && cp /kafka-healthcheck/target/release/kafka-exporter /bin

FROM debian:10.6
COPY --from=builder /bin/kafka-exporter /bin
CMD ["kafka-exporter"]
