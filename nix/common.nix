{ nixpkgs }:
pkgs:
let
  # Function to parse Cargo.toml
  parseCargoToml = cargoToml:
    let
      manifest = builtins.fromTOML (builtins.readFile cargoToml);
    in
    {
      inherit (manifest.package) name version;
    };

  # Common build inputs for all platforms
  commonBuildInputs = with pkgs; [
    llvmPackages.libclang
    pkg-config
    cmake
  ];

  # Common environment variables
  commonEnvVars = {
    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = ''-I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"'';
  };
in
{
  inherit parseCargoToml commonBuildInputs commonEnvVars;
} 