# qurli

[![Crates.io](https://img.shields.io/crates/v/qurli.svg)](https://crates.io/crates/qurli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A lightweight, terminal-based HTTP client built with Rust.

`qurli` is designed for developers, DevOps engineers, and SREs who want the power of `curl` with a clean, interactive terminal UI. It is fast, minimal, and entirely keyboard-driven.

## Features

- **HTTP Methods**: Support for GET, POST, PUT, PATCH, and DELETE.
- **Header Editor**: Dynamically add and edit request headers.
- **Body Editor**: Full support for raw JSON bodies with multiline editing.
- **Auth Support**: Easily add Bearer tokens or Basic Auth.
- **Live Curl Preview**: Generates an equivalent `curl` command in real-time as you edit.
- **Response Viewer**: View status codes, response time, headers, and pretty-printed JSON responses.
- **Request History**: Automatically saves and loads your last request state.
- **Keyboard Driven**: Optimized for speed with intuitive keybindings.

## Keybindings

- `Tab` / `Shift+Tab`: Cycle through UI sections (URL, Headers, Auth, Body, Method).
- `i` / `Enter`: Enter **Insert Mode** for the focused section.
- `Esc`: Return to **Normal Mode**.
- `m`: Cycle through HTTP methods (Normal Mode).
- `s`: Send request (Normal Mode).
- `y`: Copy generated `curl` command to clipboard (Normal Mode).
- `j` / `k`: Scroll the response body (Normal Mode).
- `q`: Quit.

## Installation

### Prerequisites

- Rust (latest stable)
- `pkg-config` and `libssl-dev` (if using system OpenSSL, though `qurli` defaults to `rustls`)

### Via Cargo

The easiest way to install `qurli` is via [crates.io](https://crates.io/crates/qurli):

```bash
cargo install qurli
```

### Build from source

```bash
git clone https://github.com/crypticani/qurli.git
cd qurli
cargo build --release
```

The binary will be available at `./target/release/qurli`.

## Tech Stack

- **Rust**: Language
- **Ratatui**: Terminal UI framework
- **Crossterm**: Terminal backend
- **Reqwest**: Async HTTP client
- **Tokio**: Async runtime
- **Serde**: Serialization/Deserialization
- **tui-textarea**: Text input handling

## License

MIT
