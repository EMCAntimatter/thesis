{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }: {

    packages = {
      default = self.nixosConfigurations.qcow;
    };

  };
}
