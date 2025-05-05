{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];

      # Function to parse Cargo.toml
      parseCargoToml =
        pkgs: cargoToml:
        let
          manifest = builtins.fromTOML (builtins.readFile cargoToml);
        in
        {
          inherit (manifest.package) name version;
        };

      # Function to get system-specific build inputs
      getBuildInputs =
        pkgs: system:
        if pkgs.stdenv.isDarwin then
          with pkgs;
          [
            darwin.apple_sdk.frameworks.CoreAudio
            darwin.apple_sdk.frameworks.AudioToolbox
            darwin.apple_sdk.frameworks.Metal
            darwin.apple_sdk.frameworks.MetalKit
            darwin.apple_sdk.frameworks.MetalPerformanceShaders
            darwin.apple_sdk.frameworks.Foundation
            darwin.apple_sdk.frameworks.AppKit
            darwin.apple_sdk.frameworks.UserNotifications
            darwin.libiconv
            openssl
          ]
        else
          with pkgs;
          [
            udev
            libinput
            alsa-lib
            alsa-utils
            openssl
            cudaPackages.cudatoolkit
            cudaPackages.cuda_cudart
            cudaPackages.cuda_nvcc
          ];

      # Function to get system-specific environment variables
      getEnvVars =
        pkgs: system:
        if pkgs.stdenv.isDarwin then
          {
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = ''-I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"'';
            CC = "${pkgs.clang}/bin/clang";
            CXX = "${pkgs.clang}/bin/clang++";
            MACOSX_DEPLOYMENT_TARGET = "11.0";
            CFLAGS = "-fmodules";
            OBJC_INCLUDE_PATH = "${pkgs.darwin.apple_sdk.frameworks.Foundation}/include:${pkgs.darwin.apple_sdk.frameworks.AppKit}/include";
            LIBRARY_PATH = "${pkgs.darwin.libiconv}/lib";
          }
        else
          {
            LD_LIBRARY_PATH = "${pkgs.llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib:${pkgs.cudaPackages.cudatoolkit}/lib";
          };

      # Function to get system-specific cargo features
      getCargoFeatures =
        pkgs: system: if pkgs.stdenv.isDarwin then "--features metal" else "--features wayland,cuda";
    in
    {
      packages = forAllSystems (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = !pkgs.stdenv.isDarwin;
            config.cudaSupport = !pkgs.stdenv.isDarwin;
          };
          # Parse Cargo.toml
          cargoMeta = parseCargoToml pkgs ./Cargo.toml;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage.override { stdenv = pkgs.clangStdenv; } (
            {
              pname = cargoMeta.name;
              version = cargoMeta.version;
              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
                outputHashes = {
                  "rdev-0.5.3" = "sha256-Ws+690+zVIp+niZ7zgbCMSKPXjioiWuvCw30faOyIrA=";
                  "whisper-rs-0.14.2" = "sha256-V+1RYWTVLHgPhRg11pz08jb3zqFtzv3ODJ1E+tf/Z9I=";
                };
              };
              cargoBuildFlags = getCargoFeatures pkgs system;
              nativeBuildInputs =
                with pkgs;
                [
                  pkg-config
                  llvmPackages.libclang
                  cmake
                ]
                ++ (if pkgs.stdenv.isDarwin then [ clang ] else [ ]);
              buildInputs = getBuildInputs pkgs system;
            }
            // (getEnvVars pkgs system)
          );
        }
      );

      devShells = forAllSystems (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = !pkgs.stdenv.isDarwin;
            config.cudaSupport = !pkgs.stdenv.isDarwin;
          };
        in
        with pkgs;
        {
          default = pkgs.mkShell.override { stdenv = clangStdenv; } (
            {
              nativeBuildInputs = [
                pkg-config
              ] ++ (if pkgs.stdenv.isDarwin then [ clang ] else [ ]);
              buildInputs = [
                rustup
                llvmPackages.libclang
                cmake
              ] ++ getBuildInputs pkgs system;
              RUST_LOG = "whispering=info";
            }
            // (getEnvVars pkgs system)
          );
        }
      );
    };
}
