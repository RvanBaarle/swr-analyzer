{pkgs, stdenv, crate2nix-tools}: let
  cargo = pkgs.callPackage (crate2nix-tools.generatedCargoNix {
    name = "swranalyzer";
    src = ./.;
  }) {
    defaultCrateOverrides = pkgs.defaultCrateOverrides // {
      libusb-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.libusb.dev ];
      };
    };
  };
in
  cargo.rootCrate.build
