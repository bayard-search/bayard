BIN_DIR ?= $(CURDIR)/bin
DATA_DIR ?= $(CURDIR)/data
CARGO_TARGET_DIR ?= $(CURDIR)/target
VERSION ?=

ifeq ($(VERSION),)
  VERSION = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="bayard") | .version')
endif

clean:
	rm -rf $(BIN_DIR)
	rm -rf $(DATA_DIR)
	rm -rf $(CARGO_TARGET_DIR)

format:
	cargo fmt

protoc:
	./generate_proto.sh

build:
	cargo update -p protobuf --precise 2.8.0
	cargo build --release
	mkdir -p $(BIN_DIR)
	cp $(CARGO_TARGET_DIR)/release/bayard $(BIN_DIR)/

docker-build:
	docker build -t bayardsearch/bayard:latest .
	docker tag bayardsearch/bayard:latest bayardsearch/bayard:$(VERSION)

docker-push:
	docker push bayardsearch/bayard:latest
	docker push bayardsearch/bayard:$(VERSION)

docker-clean:
	docker rmi -f $(shell docker images --filter "dangling=true" -q --no-trunc)
