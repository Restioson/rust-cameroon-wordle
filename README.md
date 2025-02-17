# rust-cameroon-wordle

A Wordle clone for presentation to Rust Cameroon

## Setup
First:
- Install `just`: `cargo install just`
- Install [Static Web Server](https://static-web-server.net/download-and-install/), or another of your choice
  - You can get the Linux archive here: https://github.com/static-web-server/static-web-server/releases/download/v2.36.0/static-web-server-v2.36.0-x86_64-unknown-linux-gnu.tar.gz

After this, you can try `just install-deps`. If this fails, do the following manually:
- Install the wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`
- Install wasm-bindgen: `cargo install -f wasm-bindgen-cli --version 0.2.100`

## Building
Simply run `just build`.

If you've installed Static Web Server, you can run `just run` to serve the app.

## Acknowledgements
- src/close_icon.svg and src/share_icon.svg come from Material UI (see LICENSE-MATERIAL-ICONS)
- Word list comes from https://github.com/dwyl/english-words
