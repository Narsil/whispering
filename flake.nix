{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
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
              rustup
              udev
              libinput
              alsa-lib
              alsa-utils
              openssl
              # whisper.rs
              llvmPackages.libclang
              cmake
              cudaPackages_12_4.cudatoolkit
              cudaPackages_12_4.cuda_cudart
              cudaPackages_12_4.cuda_nvcc
              SDL2
            ];
            LD_LIBRARY_PATH = "${llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib";
          };

        }
      );
    };
}
