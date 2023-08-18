FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,target=./target \
    cargo build --release && mv ./target/release/whats-for-lunch ./whats-for-lunch

FROM debian:bullseye-slim

RUN useradd -ms /bin/bash app

# Download latest listing of available packages:
RUN apt-get -y update
# Upgrade already installed packages:
RUN apt-get -y upgrade
# Install a new package:
RUN apt-get -y install curl

USER app
WORKDIR /app

COPY --from=builder /usr/src/app/whats-for-lunch /app/whats-for-lunch


CMD ./whats-for-lunch