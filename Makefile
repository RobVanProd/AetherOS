.PHONY: boot build build-rust build-initramfs run demo brain-demo forge-test clean

MUSL_TARGET := x86_64-unknown-linux-musl
FORGE_DIR := forge
ROOT := $(shell pwd)

## One-command boot: build everything + launch QEMU
boot: build run

## Build all components
build: build-rust build-initramfs

## Build Rust daemons as static musl binaries
build-rust:
	@echo "=== Building Rust daemons (static musl) ==="
	cd $(FORGE_DIR) && cargo build --target $(MUSL_TARGET) --release

## Build initramfs with BusyBox + Aether binaries
build-initramfs:
	@echo "=== Building initramfs ==="
	./tools/build_initramfs.sh

## Boot AetherOS in QEMU
run:
	./tools/run_qemu.sh

## Boot with cfcd model daemon on host (full AI stack)
demo:
	./tools/run_qemu.sh --with-cfcd

## Boot with brain server + cfcd (AI-native experience)
brain-demo:
	./tools/run_qemu.sh --with-brain

## Run Forge smoke tests (Rust)
forge-test:
	./tools/forge_smoke.sh

## Clean build artifacts
clean:
	rm -rf build/
	cd $(FORGE_DIR) && cargo clean
