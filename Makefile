# Le Coq — developer shortcuts. Rust backend + Python plugins + Svelte frontend.
# Plugin tests need Python 3.11 with plugins/requirements-dev.txt installed
# (override PYTHON=... to point at a venv).

PYTHON ?= python3.11
COMPOSE := docker compose -f docker/docker-compose.yml

.PHONY: build release test test-rust test-python lint fmt frontend frontend-dev up down logs

build:
	cargo build

release:
	cargo build --release

test: test-rust test-python

test-rust:
	cargo test

test-python:
	PYTHONPATH=. $(PYTHON) -m pytest plugins/tests/

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
	cd frontend && npx svelte-check --tsconfig ./tsconfig.json --fail-on-warnings

fmt:
	cargo fmt --all

frontend:
	cd frontend && npm ci && npm run build

frontend-dev:
	cd frontend && npm run dev

up:
	$(COMPOSE) up -d --build

down:
	$(COMPOSE) down

logs:
	$(COMPOSE) logs -f backend
