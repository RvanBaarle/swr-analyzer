{pkgs, stdenv, crate2nix-tools}: let
  cargo = pkgs.callPackage (crate2nix-tools.generatedCargoNix {
    name = "swr-analyzer";
    src = ./.;
  }) {
    defaultCrateOverrides = pkgs.defaultCrateOverrides // {
      libusb-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.libusb.dev ];
      };
      yeslogic-fontconfig-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.fontconfig.dev ];
      };
      gobject-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk3.dev ];
      };
      gio-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk3.dev ];
      };
      gdk-pixbuf-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk3.dev ];
      };
      gdk-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk3.dev ];
      };
      swr-analyzer = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.zlib.dev ];
      };
    };
  };
in
  cargo.rootCrate.build
