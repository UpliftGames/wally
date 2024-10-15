FROM rust:1.81-slim-bookworm AS build
WORKDIR /usr/app

# Debian Slim doesn't install certificates by default, but we kinda want those.
# pkg-config is used by some dependencies to locate system libraries.
RUN apt-get update
RUN apt-get install -y ca-certificates libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*

# Initialize a blank project with just our dependencies to get Docker to
# cache them. Subsequent rebuilds of the container will be able to take
# advantage of incremental compilation, which makes development much faster.
#
# We must set the USER environment variable here or else cargo init will fail.
RUN USER=root cargo new wally-registry-backend
COPY wally-registry-backend/Cargo.toml /usr/app/wally-registry-backend/
COPY Cargo.toml Cargo.lock /usr/app/
COPY src/ /usr/app/src/

RUN cargo build --package wally-registry-backend --release

# Copy actual application source in and force a modified timestamp so that
# Cargo will rebuild.
COPY ./wally-registry-backend ./wally-registry-backend/
RUN touch wally-registry-backend/src/main.rs
RUN cargo build --package wally-registry-backend --release

FROM debian:buster-slim

# Install the same SSL packages as in our build image.
RUN apt-get update
RUN apt-get install -y git ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash 1000

COPY --chown=1000 --from=build /usr/app/target/release/wally-registry-backend "/app/launch"
COPY --chown=1000 --from=build /usr/app/wally-registry-backend/Rocket.toml "/app/Rocket.toml"

USER 1000

EXPOSE 8000
WORKDIR /app
CMD ["./launch"]
