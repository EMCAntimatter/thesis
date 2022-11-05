{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    nixpkgs-mozilla = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
    nixos-generators.url = "github:nix-community/nixos-generators";
  };

  outputs = { self, nixpkgs, utils, naersk, nixpkgs-mozilla, nixos-generators }@attrs:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;

          overlays = [
            (import nixpkgs-mozilla)
            (final: prev: {
              dpdk = prev.dpdk.override {
                stdenv = prev.impureUseNativeOptimizations prev.stdenv;
              };
            })
          ];
        };
        toolchain = (pkgs.rustChannelOf {
          rustToolchain = ./rust-toolchain;
          # sha256 = "sha256-Rwm8wsq1A4MwV0y+TfBDAlpxipPTce/RaMiOxAKdiPs=";
          sha256 = "sha256-2ScxT2W4bH3TwXImNaGSJi+EajTYYkaeU5nBhYMYac4=";
          #        ^ After you run `nix build`, replace this with the actual
          #          hash from the error message
        }).rust;

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      {
        packages = {
          thesis = naersk'.buildPackage {
            name = "thesis";
            version = "0.1";
            src = ./.;
            nativeBuildInputs = [
              pkgs.pkg-config
            ];
            buildInputs = [
              pkgs.dpdk
            ] ++ pkgs.dpdk.buildInputs;
          };

          default = self.packages.${system}.thesis;

          qcow = nixos-generators.nixosGenerate {
            pkgs = nixpkgs.legacyPackages.${system};
            imports = [
              "path:vm.nix"
            ];
            format = "qcow";
          };
        };

        defaultPackage = self.packages.${system}.thesis;

        defaultApp = utils.lib.mkApp {
          drv = self.defaultPackage."${system}";
        };

        devShell = with pkgs; mkShell {
          buildInputs = [ 
            cargo rustc rustfmt pre-commit rustPackages.clippy dpdk pkg-config bash lldb valgrind
           ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };       
      });
}
