BUILD_DIR=.

run:
	cargo run

install:
	cargo build --release && cp ./target/release/cryptotrader-ticker ~/.bin/cryptick
