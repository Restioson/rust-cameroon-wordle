out_dir := "target/wasm32-unknown-unknown/debug"

install-deps:
    rustup target add wasm32-unknown-unknown
    cargo install wasm-bindgen-cli --version 0.2.100

build:
    cargo build
    wasm-bindgen --target web --no-typescript {{out_dir}}/rust_cameroon_wordle.wasm --out-dir {{out_dir}}

run: build
    static-web-server -p 8080 -d .
