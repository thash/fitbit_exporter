# Usage:
# (1). Build an image - use a custom script to workaround temporary issues during development
#    `$ ./build_docker_image.sh`
# (2). Run a container with loading the .env file
#    `$ docker run -d --env-file=.env --name fitbit_exporter --network monitoring -p 8080:8080 fitbit_exporter:latest`

FROM rust:1.68 as builder

WORKDIR /usr/src/fitbit_exporter
COPY . .
RUN cargo install --path .

# Use the official Debian image for the runtime environment
FROM debian:buster-slim

# Copy the built binary from the builder stage into the runtime container
COPY --from=builder /usr/local/cargo/bin/fitbit_exporter /usr/local/bin/fitbit_exporter
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["fitbit_exporter"]

# Expose the port used by your custom exporter
# `$ docker network create monitoring`, then add `--network monitoring` when docker run as shown top of the file
EXPOSE 8080
