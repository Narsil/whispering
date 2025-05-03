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
    in
    {
      packages = forAllSystems (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = true;
            config.cudaSupport = true;
          };
          # Parse Cargo.toml
          cargoMeta = parseCargoToml pkgs ./Cargo.toml;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = cargoMeta.name;
            version = cargoMeta.version;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "rdev-0.5.3" = "sha256-Ynj4hhi2GNj5NLzCoOJywe6uEvxhhzHfkhqc72FqHy4=";
                "whisper-rs-0.14.2" = "sha256-V+1RYWTVLHgPhRg11pz08jb3zqFtzv3ODJ1E+tf/Z9I=";

              };
            };
            nativeBuildInputs = with pkgs; [
              pkg-config
              llvmPackages.libclang
              cmake
            ];
            buildInputs = with pkgs; [
              udev
              libinput
              alsa-lib
              alsa-utils
              openssl
              cudaPackages.cudatoolkit
              cudaPackages.cuda_cudart
              cudaPackages.cuda_nvcc
            ];
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = ''-I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"'';
            CUDA_PATH = "${pkgs.cudaPackages.cudatoolkit}";
            CMAKE_CUDA_COMPILER = "${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
            CMAKE_PREFIX_PATH =
              with pkgs;
              lib.makeSearchPath "lib/cmake" [
                cudaPackages.cudatoolkit
                cudaPackages.cuda_cudart
              ];
            LD_LIBRARY_PATH =
              with pkgs;
              lib.makeLibraryPath [
                cudaPackages.cudatoolkit
                cudaPackages.cuda_cudart
                cudaPackages.libcublas
                llvmPackages.libclang.lib
              ];
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = true;
            config.cudaSupport = true;
          };
        in
        with pkgs;
        {
          default = pkgs.mkShell.override { stdenv = gcc13Stdenv; } {
            nativeBuildInputs = [
              pkg-config
            ];
            buildInputs = [
              udev
              libinput
              alsa-lib
              alsa-utils
              openssl
              rustup
              # whisper.rs
              llvmPackages.libclang
              cmake
              cudaPackages.cudatoolkit
              cudaPackages.cuda_cudart
              cudaPackages.cuda_nvcc
            ];
            LD_LIBRARY_PATH = "${llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib";
            RUST_LOG = "whispering=info";
          };
        }
      );
    };
}
