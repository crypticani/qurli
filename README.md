# qurli

[![Crates.io](https://img.shields.io/crates/v/qurli.svg)](https://crates.io/crates/qurli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A lightweight, terminal-based HTTP client built with Rust.

`qurli` is designed for developers, DevOps engineers, and SREs who want the power of `curl` with a clean, interactive terminal UI. It is fast, minimal, and entirely keyboard-driven.

## What's New in 1.1.0

- Runtime variables with `{{name}}` substitution across URLs, headers, auth, and body.
- Automatic JSON response extraction with simple paths like `$.access_token` and `$.items[0].id`.
- Secret-aware variable masking in the UI and safe curl preview.
- Safer history persistence that avoids storing runtime variables and redacts direct auth secrets.
- More robust request validation, terminal cleanup, and test coverage.

## Features

- **HTTP Methods**: Support for GET, POST, PUT, PATCH, and DELETE.
- **Header Editor**: Dynamically add and edit request headers.
- **Body Editor**: Full support for raw JSON bodies with multiline editing.
- **Auth Support**: Easily add Bearer tokens or Basic Auth.
- **Live Curl Preview**: Generates an equivalent `curl` command in real-time as you edit.
- **Response Viewer**: View status codes, response time, headers, and pretty-printed JSON responses.
- **Request History**: Automatically saves and loads your last request state.
- **Runtime Variables**: Reuse values with `{{token}}` placeholders in URLs, headers, auth, and body.
- **JSON Extraction Rules**: Extract response fields like `token = $.access_token` after a request.
- **Keyboard Driven**: Optimized for speed with intuitive keybindings.

## Keybindings

- `Tab` / `Shift+Tab`: Cycle through UI sections (URL, Headers, Auth, Body, Method).
- `i` / `Enter`: Enter **Insert Mode** for the focused section.
- `Esc`: Return to **Normal Mode**.
- `m`: Cycle through HTTP methods (Normal Mode).
- `s`: Send request (Normal Mode).
- `Ctrl+r`: Resend the current request with the latest runtime variables (Normal Mode).
- `y`: Copy generated `curl` command to clipboard (Normal Mode).
- `c`: Copy response body to clipboard (Normal Mode).
- `v`: Focus the variables/extraction panel (Normal Mode).
- `e`: Edit JSON extraction rules (Normal Mode).
- `x`: Run extraction rules manually against the latest response (Normal Mode).
- `Ctrl+n`: Clear all inputs to start a fresh request (Normal Mode).
- `j` / `k`: Scroll the response body (Normal Mode).
- `q`: Quit.

## Runtime Variables

Use `{{name}}` placeholders anywhere in the request:

```text
GET {{base_url}}/api/users/{{user_id}}
Authorization: Bearer {{token}}
```

Variables live in memory for the current session and are not persisted by default.
Secret-looking variables such as `token`, `password`, `secret`, `api_key`, and `authorization`
are masked in the UI and safe curl preview.

Before a request is sent, `qurli` substitutes variables in:

- URL
- Headers
- Auth field
- Request body

If a placeholder cannot be resolved, the request is blocked and the missing variable name is shown
in the response/status area instead of sending a partial request.

## JSON Extraction

Add extraction rules in the Extract panel:

```text
token = $.access_token
user_id = $.user.id
first_item = $.items[0].id
```

After each successful JSON response, `qurli` runs the rules and stores matching values as
runtime variables. Missing paths are ignored, and invalid JSON is reported in the status bar.

Example login flow:

1. Send a login request.
2. Add `token = $.access_token`.
3. Send the request and let `qurli` extract the token.
4. Use `Authorization: Bearer {{token}}` in the next request.

The on-screen curl preview masks secret variables. Pressing `y` copies the real substituted
curl command, so avoid doing that in shared terminals when secrets are present.

Supported extraction syntax is intentionally small:

- `$.field`
- `$.nested.field`
- `$.items[0].id`

## History and Secrets

`qurli` stores the last request template under your user config directory. Runtime variables
are not written to history. Direct `Authorization` and auth field values are redacted before
history is saved, and JSON body fields with secret-looking names are redacted where possible.

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
