{                                                                           
  inputs = {
    utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, fenix, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system;
                                overlays = [ fenix.overlays.default ];
                              };
        lib = nixpkgs.lib;
      in
      {
        devShells.default = pkgs.mkShell {
#          stdenv = pkgs.llvmPackages_17.stdenv;
          nativeBuildInputs =
            with pkgs; [
              fenix.packages.${system}.complete.toolchain
              rust-analyzer-nightly
              vulkan-loader
              vulkan-headers              
              vulkan-tools
              # Package location
				      pkg-config
				      # Window and Input
#				      x11
				      xorg.libXcursor
				      xorg.libXi
				      vulkan-validation-layers
              pipewire
#              nvidia-settings
              libdrm
              libxkbcommon
              xorg.libXext
              xorg.libX11
              xorg.libXv
              xorg.libXrandr
              xorg.libxcb
              zlib
#              stdenv.cc.cc
              wayland
              mesa
              libGL
              openssl
              dbus # for nvidia-powerd
              alsaLib # Sound support
				      udev # device management
#              clangStdenv
              llvmPackages_17.stdenv
              llvmPackages_17.stdenv.cc
#				      lld # fast linker
            ];
          APPEND_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
            libxkbcommon
            vulkan-loader
            xorg.libX11
            xorg.libxcb
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ];
          
          shellHook = ''
      export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$APPEND_LIBRARY_PATH"
    '';      
        };
      });
}
