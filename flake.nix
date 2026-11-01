{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [rust-overlay.overlays.default];
    };
    rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = [
        rustToolchain
      ];
    };
  };
}
