# Targets:
#   make          - build ./tiny-business-simulator (release)
#   make test     - run the unit tests (cargo test)
#   make run      - build and run ./tiny-business-simulator on the local test_data folder
#   make install - copy ./tiny-business-simulator to /usr/local/bin
#   make musl    - build a static binary linked with musl
#   make clean    - remove build artifacts and ./tiny-business-simulator

.PHONY: all build test run install musl clean

all: build

build:
	cargo build --release
	cp target/release/tiny-business-simulator tiny-business-simulator

test:
	cargo test

run: build
	./tiny-business-simulator sample_business

install: build
	cp tiny-business-simulator /usr/local/bin/tiny-business-simulator

musl:
	cargo build --release --target x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/tiny-business-simulator tiny-business-simulator

clean:
	cargo clean
	rm -f tiny-business-simulator test_runner
