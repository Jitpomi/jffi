.PHONY: help build run dev clean

help:
	@echo "UniFFI App - Build Commands"
	@echo ""
	@echo "  make build PLATFORM=<platform>  - Build for platform"
	@echo "  make run PLATFORM=<platform>    - Run on platform"
	@echo "  make dev PLATFORM=<platform>    - Watch mode"
	@echo "  make clean                      - Clean build artifacts"
	@echo ""
	@echo "Available platforms: linux"
	@echo "Default platform: linux"

PLATFORM ?= linux

build:
	@uniffi-app build --platform $(PLATFORM)

run:
	@uniffi-app run --platform $(PLATFORM)

dev:
	@uniffi-app dev --platform $(PLATFORM)

clean:
	@cargo clean
	@echo "✅ Cleaned build artifacts"
