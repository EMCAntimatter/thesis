.PHONY: build
build: nix-clean
	nix-build

.PHONY: test
test: build
	sudo nix-shell --command "cd dpdk-hello-world && ../result/bin/dpdk-hello-world"

.PHONY: nix-clean
nix-clean:
	sudo nix-store --delete --ignore-liveness ./result

.PHONY: container
container: ./flake.nix stop-container
	sudo nixos-container create --flake ./flake.nix thesis

.PHONY: stop-container
stop-container:
	sudo nixos-container destroy thesis
	