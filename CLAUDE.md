# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

- `cargo build` — build debug binary
- `cargo build --release` — build optimized release binary
- `cargo build --profile minsize` — build size-optimized binary (stripped, abort on panic)
- `cargo clippy` — lint
- `cargo fmt` — format (120 char width, crate-level import granularity, std/external/crate grouping)

There are no tests in this project.

## Project Overview

CLI tool that fetches the latest temperature reading from the Finnish Meteorological Institute (FMI) open data API. It queries the WFS XML endpoint by station ID (fmisid), parses the XML response with `roxmltree` to extract the most recent `MeasurementTVP` time-value pair, and prints the temperature to stdout.

Optionally forwards the reading via MQTT publish (`rumqttc`) and/or CoAP POST (`coap` crate), run concurrently with `tokio::join!`.

## Architecture

- **`src/bin/fmi_get_temp.rs`** — Binary entrypoint. Parses CLI opts, calls `get_temp()`, prints result, then fans out MQTT/CoAP sends concurrently.
- **`src/lib.rs`** — Core logic: `get_temp()` (FMI API fetch + XML parsing), `coap_send()`, `mqtt_send()`. Re-exports common deps for use via `use fmi_get_temp::*`.
- **`src/config.rs`** — `OptsCommon` struct (clap derive) with all CLI flags/options. Also provides `get_loglevel()` and `start_pgm()` for tracing init.
- **`src/web_util.rs`** — HTTP client helpers using `reqwest` with rustls. `get_body()` / `get_text_body()` with timeouts.
- **`build.rs`** — Embeds git branch, commit, source timestamp, and rustc version via `build-data` crate.

## Key Conventions

- Rust edition 2024, stable toolchain.
- Uses `anyhow` for error handling throughout.
- The FMI URL template uses `###FMI_SID###` and `###START_TIME###` as placeholders, replaced at runtime in `get_temp()`.
- Logging via `tracing`; verbosity controlled by `-v`/`-d`/`-t` flags (info/debug/trace).
- `foo` file at repo root is a sample FMI XML response, useful for understanding the parsed structure.
