{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, crane, fenix, ... }:
    let 
      system = "x86_64-linux";
      packageName = "fw-lights";
      pkgs = import nixpkgs {
        inherit system;
      };
      rust-pkgs = fenix.packages.${system};
      rust = (rust-pkgs.stable.withComponents [
        "rustc"
        "cargo"
        "clippy"
        "rustfmt"
        "rust-std"
        "rust-src"
        "rust-analyzer"
      ]);
      crane-lib = (crane.mkLib pkgs).overrideToolchain rust;
    in rec {
      packages.${system} = rec {
        default = crane-lib.buildPackage {
          src = ./.;
          buildInputs = with pkgs; [ systemd ];
          nativeBuildInputs = with pkgs; [ pkg-config ];
        };
        ${packageName} = default;
      };

      checks.${system}.${packageName} = packages.${system}.default;

      devShells.${system}.default = crane-lib.devShell {
        checks = checks.${system};
      };
    };
}
