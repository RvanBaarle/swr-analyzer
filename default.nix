{pkgs, stdenv, crate2nix-tools}: let
  cargo = pkgs.callPackage (crate2nix-tools.generatedCargoNix {
    name = "swr-analyzer";
    src = ./.;
  }) {
    defaultCrateOverrides = pkgs.defaultCrateOverrides // {
      libusb1-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.libusb.dev ];
      };
      yeslogic-fontconfig-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.fontconfig.dev ];
      };
      gobject-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk4.dev ];
      };
      gio-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk4.dev ];
      };
      gdk-pixbuf-sys = attrs: {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.gtk4.dev ];
      };
    };
  };
in
  cargo.rootCrate.build.overrideAttrs {
    postInstall = ''
      mkdir -p $out/etc/udev/rules.d
      cp ./udev/* $out/etc/udev/rules.d/
    '';
  }
