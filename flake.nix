{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    crate2nix.url = "github:nix-community/crate2nix";
    crate2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, crate2nix }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    crate2nix-tools = crate2nix.tools.${system};
  in {

    packages.${system} = rec {
      swranalyzer = pkgs.callPackage ./default.nix { inherit crate2nix-tools; };
      default = swranalyzer;
    };

    devShell.${system} = pkgs.mkShell {
      packages = with pkgs; [ clang libusb.dev pkg-config ];
    };

  };
}
