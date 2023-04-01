#!/bin/bash
#
# As of the date of writing, I'm using a locally modified version of `prometheus-client` (https://github.com/prometheus/client_rust) because of a set of missing features.
#
# ```Cargo.toml
# # prometheus-client = "0.19.0"
# prometheus-client = { path = "dependencies/client_rust" }
# ```
#
# ```
# $ tree . -L 2
# .
# ├── Cargo.lock
# ├── Cargo.toml
# ├── Dockerfile
# ├── config
# │   └── prometheus.yml
# ├── dependencies
# │   └── client_rust -> /path/to/local/cloned/prometheus/client_rust
# ├── docker_build.sh
# ├── fitbit_historical_metrics.prom
# ├── grafana_dashboard.json
# ├── src
# │   ├── fitbit
# │   ├── lib.rs
# │   └── main.rs
# └── target
#     ├── CACHEDIR.TAG
#     ├── debug
#     └── release
# ```
#
# (A). `cargo run` follow symlink. No problem.
# (B). `docker build` doesn't follow symlink. To workaround it, I have created this script.

# Remember the target of the symlink
SYMLINK_TARGET=$(readlink dependencies/client_rust)

# Remove the symlink (don't delete the target repository itself, just remove symlink)
rm dependencies/client_rust

# Copy from the target recursively, ignoring "target" and "Cargo.lock"
rsync -a --exclude target --exclude Cargo.lock "$SYMLINK_TARGET"/ dependencies/client_rust

# Execute docker build
docker build -t fitbit_exporter:latest .

# Remove the copied directory
rm -rf dependencies/client_rust

# Re-create the symlink with the saved target location
ln -s "$SYMLINK_TARGET" dependencies/client_rust

