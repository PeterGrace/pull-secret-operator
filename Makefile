all: build

build: TAG=$(shell git describe --abbrev=0)
build: HASH=$(shell git rev-parse --short HEAD)
build:
	docker run --rm --user "$(shell id -u)":"$(shell id -g)" -v $(shell pwd):/usr/src/myapp -w /usr/src/myapp rustlang/rust:nightly cargo +nightly build 
	docker build -t petergrace/pull-secret-operator:$(TAG) .


bump:
	cargo bump -g
	$(eval TAG=`git describe --abbrev=0`)
	yq w -i chart/pull-secret-operator/Chart.yaml appVersion $(TAG)
	git add chart/pull-secret-operator/Chart.yaml
	git commit -m "synchronizing chart appVer with current tag: $(TAG)"

gh-build: TAG=$(shell git describe --abbrev=0)
gh-build: HASH=$(shell git rev-parse --short HEAD)
gh-build:
	cargo +nightly build --release
	docker build -t petergrace/pull-secret-operator:$(HASH) .
	docker tag petergrace/pull-secret-operator:$(HASH) petergrace/pull-secret-operator:$(TAG)
	docker tag petergrace/pull-secret-operator:$(HASH) petergrace/pull-secret-operator:latest 
	docker push petergrace/pull-secret-operator:$(HASH)
	docker push petergrace/pull-secret-operator:$(TAG)
	docker push petergrace/pull-secret-operator:latest
