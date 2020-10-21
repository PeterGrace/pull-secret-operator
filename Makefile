all: build

build:
	docker run --rm --user "$(shell id -u)":"$(shell id -g)" -v $(shell pwd):/usr/src/myapp -w /usr/src/myapp rustlang/rust:nightly cargo build --release
	docker build -t foo .
