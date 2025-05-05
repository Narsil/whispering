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
            wayland
            wayland-protocols
            wayland-scanner
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
            CUDA_PATH = "${pkgs.cudaPackages.cudatoolkit}";
            EXTRA_LDFLAGS = "-L${pkgs.cudaPackages.cudatoolkit}/lib/stubs";
            CUDA_TOOLKIT_ROOT_DIR = "${pkgs.cudaPackages.cudatoolkit}";
            CMAKE_CUDA_COMPILER = "${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
          };

      # Function to get system-specific cargo features
      getCargoFeatures =
        pkgs: system:
        if pkgs.stdenv.isDarwin then
          [ "metal" ]
        else
          [
            "wayland"
            "cuda"
          ];
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
              buildFeatures = getCargoFeatures pkgs system;
              CMAKE_ARGS =
                if pkgs.stdenv.isDarwin then
                  ""
                else
                  "-DCMAKE_CUDA_COMPILER=${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc -DCMAKE_CUDA_ARCHITECTURES=all -DCUDA_TOOLKIT_ROOT_DIR=${pkgs.cudaPackages.cudatoolkit} -DCUDA_INCLUDE_DIRS=${pkgs.cudaPackages.cudatoolkit}/include -DCUDA_CUDART_LIBRARY=${pkgs.cudaPackages.cuda_cudart}/lib/libcudart.so -DCUDA_NVCC_EXECUTABLE=${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
              nativeBuildInputs =
                with pkgs;
                [
                  pkg-config
                  cmake
                ]
                ++ (if pkgs.stdenv.isDarwin then [ clang ] else [ cudaPackages.cuda_nvcc ]);
              buildInputs =
                [
                  pkgs.llvmPackages.libclang
                ]
                ++ (
                  if pkgs.stdenv.isDarwin then
                    [ ]
                  else
                    with pkgs.cudaPackages;
                    [
                      cuda_cudart
                      cuda_nvcc
                      cudatoolkit
                    ]
                )
                ++ getBuildInputs pkgs system;
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
              nativeBuildInputs =
                with pkgs;
                [
                  pkg-config
                  cmake
                ]
                ++ (if pkgs.stdenv.isDarwin then [ clang ] else [ cudaPackages.cuda_nvcc ]);
              buildInputs = [
                rustup
                llvmPackages.libclang
              ] ++ getBuildInputs pkgs system;
              RUST_LOG = "whispering=info";
            }
            // (getEnvVars pkgs system)
          );
        }
      );
    };
}
