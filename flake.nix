{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
    in
    {
      packages = forAllSystems (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = !pkgs.stdenv.isDarwin;
            config.cudaSupport = !pkgs.stdenv.isDarwin;
          };
          package = pkgs.callPackage ./nix/package.nix {
            inherit (pkgs) lib stdenv rustPlatform pkg-config cmake llvmPackages openssl darwin udev libinput alsa-lib alsa-utils wayland wayland-protocols wayland-scanner xorg cudaPackages;
          };
        in
        {
          default = if pkgs.stdenv.isDarwin
            then package.darwin
            else package.linux-wayland;
          inherit (package) darwin linux-wayland linux-x11;
        }
      );

      devShells = forAllSystems (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = !pkgs.stdenv.isDarwin;
            config.cudaSupport = !pkgs.stdenv.isDarwin;
          };
          package = pkgs.callPackage ./nix/package.nix {
            inherit (pkgs) lib stdenv rustPlatform pkg-config cmake llvmPackages openssl darwin udev libinput alsa-lib alsa-utils wayland wayland-protocols wayland-scanner xorg cudaPackages;
          };
          variant = if pkgs.stdenv.isDarwin then package.darwin else package.linux-wayland;
        in
        with pkgs;
        {
          default = mkShell {
            inputsFrom = [ variant ];
            nativeBuildInputs = [
              rustup
              clang
              llvmPackages.libclang
              gcc
              pkg-config
              cmake
            ];
            buildInputs = [
              openssl
              udev
              libinput
              alsa-lib
              alsa-utils
              wayland
              wayland-protocols
              wayland-scanner
              xorg.libX11
              xorg.libXcursor
              xorg.libXrandr
              xorg.libXi
            ] ++ (if !stdenv.isDarwin then [
              cudaPackages.cudatoolkit
              cudaPackages.cuda_cudart
              cudaPackages.cuda_nvcc
            ] else [
              darwin.apple_sdk.frameworks.CoreAudio
              darwin.apple_sdk.frameworks.AudioToolbox
              darwin.apple_sdk.frameworks.Metal
              darwin.apple_sdk.frameworks.MetalKit
              darwin.apple_sdk.frameworks.MetalPerformanceShaders
              darwin.apple_sdk.frameworks.Foundation
              darwin.apple_sdk.frameworks.AppKit
              darwin.apple_sdk.frameworks.UserNotifications
              darwin.libiconv
            ]);
            shellHook = ''
              export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib"
              export BINDGEN_EXTRA_CLANG_ARGS="-I${llvmPackages.libclang.lib}/lib/clang/${llvmPackages.libclang.version}/include"
              ${if !stdenv.isDarwin then ''
                export LD_LIBRARY_PATH="${llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib:${cudaPackages.cudatoolkit}/lib"
                export CUDA_PATH="${cudaPackages.cudatoolkit}"
                export EXTRA_LDFLAGS="-L${cudaPackages.cudatoolkit}/lib/stubs"
                export CUDA_TOOLKIT_ROOT_DIR="${cudaPackages.cudatoolkit}"
                export CMAKE_CUDA_COMPILER="${cudaPackages.cuda_nvcc}/bin/nvcc"
              '' else ''
                export CC="${stdenv.cc}/bin/clang"
                export CXX="${stdenv.cc}/bin/clang++"
                export MACOSX_DEPLOYMENT_TARGET="11.0"
                export CFLAGS="-fmodules"
                export OBJC_INCLUDE_PATH="${darwin.apple_sdk.frameworks.Foundation}/include:${darwin.apple_sdk.frameworks.AppKit}/include"
                export LIBRARY_PATH="${darwin.libiconv}/lib"
              ''}
            '';
            RUST_LOG = "whispering=info";
          };
        }
      );
    };
}
