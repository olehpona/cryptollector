FROM rust:alpine AS builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM alpine:3
LABEL authors="oleh"
LABEL org.opencontainers.image.source="https://github.com/olehpona/paymenator"

RUN mkdir /app

COPY --from=builder /usr/local/cargo/bin/paymenator /app/paymenator
COPY .env /app/.env

WORKDIR /app

CMD ["/app/paymenator"]