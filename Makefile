# Targets:
#   make                  - build ./tiny-business-simulator (native release)
#   make test             - run the unit tests (cargo test)
#   make run              - build and run ./tiny-business-simulator on sample_business
#   make install          - copy ./tiny-business-simulator to /usr/local/bin
#   make clean            - remove build artifacts, ./tiny-business-simulator and dist/
#
#   make targets          - install every rustup target used below
#   make install-cross    - install `cross` (Docker) for linux cross/musl builds
#
#   Release artifacts (mirrors .github/workflows/release.yml, output in dist/):
#   make linux            - x86_64-unknown-linux-gnu     -> dist/...x86_64_linux_gnu.tar.gz
#   make linux-arm64      - aarch64-unknown-linux-gnu    -> dist/...aarch64_linux_gnu.tar.gz
#   make musl             - x86_64-unknown-linux-musl    -> dist/...x86_64_linux_musl.tar.gz
#   make musl-arm64       - aarch64-unknown-linux-musl   -> dist/...aarch64_linux_musl.tar.gz
#   make macos-intel      - x86_64-apple-darwin          -> dist/...x86_64_macos.tar.gz
#   make macos-arm        - aarch64-apple-darwin         -> dist/...aarch64_macos.tar.gz
#   make windows          - x86_64-pc-windows-msvc       -> dist/...x86_64_windows.zip
#   make windows-arm64    - aarch64-pc-windows-msvc      -> dist/...aarch64_windows.zip
#   make dist             - build & archive every target above
#
#   linux cross/musl builds use `cross` (Docker). macOS/Windows builds use native
#   cargo and must run on the matching host OS.

BIN  := tiny-business-simulator
DIST := dist

RUST_TARGETS := x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
               x86_64-unknown-linux-musl aarch64-unknown-linux-musl \
               x86_64-apple-darwin aarch64-apple-darwin \
               x86_64-pc-windows-msvc aarch64-pc-windows-msvc

.PHONY: all build test run install clean targets install-cross dist \
        linux linux-arm64 musl musl-arm64 macos-intel macos-arm windows windows-arm64

all: build

build:
	cargo build --release
	cp target/release/$(BIN) $(BIN)

test:
	cargo test

run: build
	./$(BIN) sample_business

install: build
	cp $(BIN) /usr/local/bin/$(BIN)

clean:
	cargo clean
	rm -f $(BIN) test_runner
	rm -rf $(DIST)

targets:
	rustup target add $(RUST_TARGETS)

install-cross:
	cargo install cross

# --- archived release artifacts ---

# $(1) = rust target triple, $(2) = output archive name (dist/...)
define archive-tar
	@mkdir -p $(DIST) .staging
	cp target/$(1)/release/$(BIN) .staging/$(BIN)
	chmod +x .staging/$(BIN)
	tar -czf $(DIST)/$(2) -C .staging $(BIN)
	@rm -rf .staging
endef

# $(1) = rust target triple, $(2) = output archive name (dist/...)
define archive-zip
	@mkdir -p $(DIST) .staging
	cp target/$(1)/release/$(BIN).exe .staging/$(BIN).exe
	zip -j $(DIST)/$(2) .staging/$(BIN).exe
	@rm -rf .staging
endef

linux:
	cargo build --release --locked --target x86_64-unknown-linux-gnu
	$(call archive-tar,x86_64-unknown-linux-gnu,tiny-business-simulator-x86_64_linux_gnu.tar.gz)

linux-arm64:
	cross build --release --locked --target aarch64-unknown-linux-gnu
	$(call archive-tar,aarch64-unknown-linux-gnu,tiny-business-simulator-aarch64_linux_gnu.tar.gz)

musl:
	cross build --release --locked --target x86_64-unknown-linux-musl
	$(call archive-tar,x86_64-unknown-linux-musl,tiny-business-simulator-x86_64_linux_musl.tar.gz)

musl-arm64:
	cross build --release --locked --target aarch64-unknown-linux-musl
	$(call archive-tar,aarch64-unknown-linux-musl,tiny-business-simulator-aarch64_linux_musl.tar.gz)

macos-intel:
	cargo build --release --locked --target x86_64-apple-darwin
	$(call archive-tar,x86_64-apple-darwin,tiny-business-simulator-x86_64_macos.tar.gz)

macos-arm:
	cargo build --release --locked --target aarch64-apple-darwin
	$(call archive-tar,aarch64-apple-darwin,tiny-business-simulator-aarch64_macos.tar.gz)

windows:
	cargo build --release --locked --target x86_64-pc-windows-msvc
	$(call archive-zip,x86_64-pc-windows-msvc,tiny-business-simulator-x86_64_windows.zip)

windows-arm64:
	cargo build --release --locked --target aarch64-pc-windows-msvc
	$(call archive-zip,aarch64-pc-windows-msvc,tiny-business-simulator-aarch64_windows.zip)

dist: linux linux-arm64 musl musl-arm64 macos-intel macos-arm windows windows-arm64
	@ls -lh $(DIST)
