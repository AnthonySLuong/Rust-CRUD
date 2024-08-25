FROM rust:latest AS builder
WORKDIR /app
ADD . /app
RUN cargo build --release

FROM gcr.io/distroless/cc
LABEL authors="anthonyluong"
COPY --from=builder /app/target/release/rust-crud-api /
CMD ["./rust-crud-api"]

