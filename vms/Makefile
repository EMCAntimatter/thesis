nixos.qcow2: vm.nix
	nix build .#qcow

nixos.ami: vm.nix
	nix build .#ami

.PHONY: run-vm
run-vm:
	nixos-generate -f vm -c ./vm.nix --run