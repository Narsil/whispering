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

      # NixOS module
      nixosModule = { config, lib, pkgs, ... }:
        let
          cfg = config.services.whispering;
        in
        {
          options.services.whispering = {
            enable = lib.mkEnableOption "Whispering service";

            package = lib.mkOption {
              type = lib.types.package;
              default = pkgs.whispering;
              description = "The Whispering package to use.";
            };

            user = lib.mkOption {
              type = lib.types.str;
              default = "whispering";
              description = "User account under which Whispering runs.";
            };

            group = lib.mkOption {
              type = lib.types.str;
              default = "whispering";
              description = "Group under which Whispering runs.";
            };

            dataDir = lib.mkOption {
              type = lib.types.path;
              default = "/var/lib/whispering";
              description = "Directory to store Whispering data.";
            };

            # Application configuration
            settings = {
              # Audio settings
              audio = {
                channels = lib.mkOption {
                  type = lib.types.ints.u16;
                  default = 1;
                  description = "Number of audio channels (1 for mono, 2 for stereo).";
                };
                sample_rate = lib.mkOption {
                  type = lib.types.ints.u32;
                  default = 16000;
                  description = "Sample rate in Hz.";
                };
                bits_per_sample = lib.mkOption {
                  type = lib.types.ints.u16;
                  default = 32;
                  description = "Bits per sample.";
                };
                sample_format = lib.mkOption {
                  type = lib.types.enum [ "float" "int" ];
                  default = "float";
                  description = "Sample format (float or int).";
                };
              };

              # Model settings
              model = {
                repo = lib.mkOption {
                  type = lib.types.str;
                  default = "ggerganov/whisper.cpp";
                  description = "Hugging Face model repository.";
                };
                filename = lib.mkOption {
                  type = lib.types.str;
                  default = "ggml-base.en.bin";
                  description = "Model filename.";
                };
              };

              # Path settings
              paths = {
                cache_dir = lib.mkOption {
                  type = lib.types.path;
                  default = "${cfg.dataDir}/cache";
                  description = "Cache directory for storing temporary files.";
                };
                recording_path = lib.mkOption {
                  type = lib.types.path;
                  default = "${cfg.dataDir}/cache/recorded.wav";
                  description = "Path to the recorded audio file.";
                };
              };

              # Shortcut settings
              shortcuts = {
                keys = lib.mkOption {
                  type = lib.types.listOf lib.types.str;
                  default = [ "ControlLeft" "Space" ];
                  description = "Keys that need to be pressed in sequence to start recording.";
                };
                autosend = lib.mkOption {
                  type = lib.types.bool;
                  default = false;
                  description = "Automatically hit enter after sending the text.";
                };
              };
            };

            # Additional environment variables
            environment = lib.mkOption {
              type = lib.types.attrsOf lib.types.str;
              default = { };
              description = "Additional environment variables for the service.";
            };

            # Additional systemd service settings
            serviceConfig = lib.mkOption {
              type = lib.types.attrsOf lib.types.anything;
              default = { };
              description = "Additional systemd service configuration.";
            };
          };

          config = lib.mkIf cfg.enable {
            users.users = lib.optionalAttrs (cfg.user == "whispering") {
              whispering = {
                isSystemUser = true;
                group = cfg.group;
                home = cfg.dataDir;
                createHome = true;
              };
            };

            users.groups = lib.optionalAttrs (cfg.group == "whispering") {
              whispering = { };
            };

            # Create configuration file
            environment.etc."whispering/config.toml" = {
              text = builtins.toTOML {
                audio = cfg.settings.audio;
                model = cfg.settings.model;
                paths = cfg.settings.paths;
                shortcuts = cfg.settings.shortcuts;
              };
              mode = "0644";
            };

            systemd.services.whispering = {
              description = "Whispering service";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];
              serviceConfig = {
                Type = "simple";
                User = cfg.user;
                Group = cfg.group;
                WorkingDirectory = cfg.dataDir;
                ExecStart = "${cfg.package}/bin/whispering --config /etc/whispering/config.toml";
                Restart = "on-failure";
                RestartSec = "10s";
                # Required for CUDA and audio
                SupplementaryGroups = [ "audio" "video" "input" ];
                # Required for Wayland
                Environment = [
                  "XDG_RUNTIME_DIR=/run/user/1000"
                  "WAYLAND_DISPLAY=wayland-0"
                ] ++ (lib.mapAttrsToList (name: value: "${name}=${value}") cfg.environment);
                # Additional service configuration
              } // cfg.serviceConfig;
            };
          };
        };
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
                ]
                ++ (if pkgs.stdenv.isDarwin then [ clang ] else [ ]);
              buildInputs = [
                pkgs.llvmPackages.libclang
                pkgs.cmake
              ] ++ getBuildInputs pkgs system;
            }
            // (getEnvVars pkgs system)
          );
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

      # Add NixOS module
      nixosModules.default = nixosModule;

      # Add overlay to make the package available in nixpkgs
      overlays.default = final: prev: {
        whispering = final.callPackage ./nix/package.nix { };
      };
    };
}
