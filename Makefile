all: build

build: VER=$(shell git rev-parse --short HEAD)
build:
	docker run --rm --user "$(shell id -u)":"$(shell id -g)" -v $(shell pwd):/usr/src/myapp -w /usr/src/myapp rustlang/rust:nightly cargo +nightly build 
	docker build -t petergrace/pull-secret-operator:test .

gh-build: VER=$(shell git rev-parse --short HEAD)
gh-build:
	cargo +nightly build --release
	docker build -t petergrace/pull-secret-operator:$(VER) .
	docker tag petergrace/pull-secret-operator:$(VER) petergrace/pull-secret-operator:latest 
	docker push petergrace/pull-secret-operator:$(VER)
	docker push petergrace/pull-secret-operator:latest
