.PHONY:

all: .PHONY
	cargo build --release --target wasm32-unknown-unknown
	wasm-bindgen --out-dir ./out/ --target web ./target/wasm32-unknown-unknown/release/bevy_js_cmd.wasm
serve: all
	basic-http-server .
