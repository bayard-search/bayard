BIN_DIR ?= $(CURDIR)/bin
DOCS_DIR ?= $(CURDIR)/docs
VERSION ?=

ifeq ($(VERSION),)
  VERSION = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="bayard") | .version')
endif

.DEFAULT_GOAL := build

clean:
	rm -rf $(BIN_DIR)
	cargo clean

format:
	cargo fmt

build:
	mkdir -p $(BIN_DIR)
	cargo build --release
	cp -p ./target/release/bayard $(BIN_DIR)

test:
	cargo test

build-docker:
	docker build -t bayardsearch/bayard:latest .
	docker tag bayardsearch/bayard:latest bayardsearch/bayard:$(VERSION)

push-docker:
	docker push bayardsearch/bayard:latest
	docker push bayardsearch/bayard:$(VERSION)

clean-docker:
	docker rmi -f $(shell docker images --filter "dangling=true" -q --no-trunc)

.PHONY: docs
docs:
	rm -rf $(DOCS_DIR)
	mdbook build
