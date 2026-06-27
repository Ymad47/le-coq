.PHONY: build test clean run dev

build:
	zig build

test:
	zig build test

clean:
	rm -rf zig-out zig-cache

run:
	zig build run

dev:
	zig build run -- --dev

# Release builds
release:
	zig build -Doptimize=ReleaseFast

release-safe:
	zig build -Doptimize=ReleaseSafe

# Install locally
install:
	zig build --prefix /usr/local

# Docker (future)
docker-build:
	docker build -t la-coq/apollo:latest .

docker-run:
	docker run -p 8080:8080 la-coq/apollo:latest
