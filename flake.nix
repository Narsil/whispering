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
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = with pkgs; [
              pkg-config
              llvmPackages.libclang
              cmake
              cudaPackages.cudatoolkit
              cudaPackages.cuda_cudart
              cudaPackages.cuda_nvcc
            ];
            buildInputs = with pkgs; [
              udev
              libinput
              alsa-lib
              alsa-utils
              openssl
            ];
            # LD_LIBRARY_PATH = "${pkgs.llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib";
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
              cudaPackages_12_4.cudatoolkit
              cudaPackages_12_4.cuda_cudart
              cudaPackages_12_4.cuda_nvcc
            ];
            LD_LIBRARY_PATH = "${llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib";
            RUST_LOG = "whispering=info";
          };
        }
      );
    };
}
