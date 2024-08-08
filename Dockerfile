# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.80.0
ARG APP_NAME=chatd

# Create a stage for building the application.
FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache make clang lld musl-dev git

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies, a cache mount to /usr/local/cargo/git/db
# for git repository dependencies, and a cache mount to /app/target/ for
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the src directory to avoid having to copy the
# source code into the container.
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=Makefile,target=Makefile \
    --mount=type=bind,source=motd.ans,target=motd.ans \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    make release && \
    cp ./target/release/$APP_NAME /bin/server


FROM alpine:3.18 AS final

# Generate a server's SSH key
RUN apk add openssh
RUN mkdir /root/.ssh
WORKDIR /root/.ssh
RUN ssh-keygen -t rsa -C "chatkey" -f id_rsa

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

# Expose the port that the application listens on.
EXPOSE 2022

# What the container should run when it is started.
CMD ["/bin/server", "--port", "2022", "-i", "/root/.ssh/id_rsa"]
