# Fitbit Exporter

Fitbit Exporter is a custom Prometheus exporter designed to collect and expose health and fitness data from Fitbit's API. It allows users to integrate Fitbit metrics such as steps and sleep patterns into their Prometheus monitoring setup, making it an ideal tool for personal health data analytics.

## Project Status

Important Notice: This repository is currently a work in progress and is not ready for use. Due to my personal development priorities, this repository will not be updated anymore. Please be aware that the code may be incomplete and not suitable for production use.

## File Structure

- `src/`
  - `fitbit/`: Module containing the core functionality.
    - `client.rs`: Handles API interactions with Fitbit.
    - `cmd.rs`: Command-line interface handling.
    - `history.rs`: Functions for historical data processing.
    - `metrics.rs`: Metrics collection and processing.
    - `server.rs`: Server setup for Prometheus scraping.
  - `main.rs`: Entry point of the application.
- `grafana_dashboard.json`: A Grafana dashboard configuration for visualizing the metrics.
- `dependencies`: Folder containing a custom version of client_rust (not included in the repo).
- `build_docker_image.sh`: Script to build the Docker image.
- `Dockerfile`: Instructions for building the Docker image.
- `config/prometheus.yml`: Prometheus configuration file.

