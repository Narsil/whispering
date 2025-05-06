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

      # Build the package
      buildPackage =
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = !pkgs.stdenv.isDarwin;
            config.cudaSupport = !pkgs.stdenv.isDarwin;
          };
          pkg = pkgs.callPackage ./nix/package.nix { };
        in
        if pkgs.stdenv.isDarwin then pkg.darwin else pkg.linux-wayland;

      # NixOS module
      nixosModule =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.whispering;
          # Detect display server
          isWayland = true;
          # Get display server specific environment variables
          displayEnv =
            if isWayland then
              [
                "XDG_RUNTIME_DIR=/run/user/1000"
                "WAYLAND_DISPLAY=wayland-0"
              ]
            else
              [
                "DISPLAY=:0"
                "XAUTHORITY=/home/${cfg.user}/.Xauthority"
              ];
          # Get display server specific cargo features
          displayFeatures = if isWayland then "wayland" else "x11";
          # Create TOML format
          tomlFormat = pkgs.formats.toml { };
        in
        {
          options.services.whispering = {
            enable = lib.mkEnableOption "Whispering service";

            package = lib.mkOption {
              type = lib.types.package;
              default = buildPackage pkgs.system;
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
                  type = lib.types.enum [
                    "float"
                    "int"
                  ];
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
                prompt = lib.mkOption {
                  type = lib.types.attrsOf lib.types.anything;
                  default = {
                    type = "none";
                  };
                  description = "Prompt configuration for the model.";
                  example = {
                    type = "vocabulary";
                    vocabulary = [
                      "word1"
                      "word2"
                    ];
                  };
                };
                replacements = lib.mkOption {
                  type = lib.types.attrsOf lib.types.str;
                  default = { };
                  description = "Map of text to replace with their replacements.";
                  example = {
                    "incorrect text" = "correct text";
                    "another mistake" = "another correction";
                  };
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
                  default = [
                    "ControlLeft"
                    "Space"
                  ];
                  description = "Keys that need to be pressed in sequence to start recording.";
                };
                autosend = lib.mkOption {
                  type = lib.types.bool;
                  default = false;
                  description = "Automatically hit enter after sending the text.";
                };
              };

              # Vocabulary settings
              vocabulary = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "List of words to improve recognition accuracy.";
              };

              # Replacement settings
              replacements = lib.mkOption {
                type = lib.types.attrsOf lib.types.str;
                default = { };
                description = "Map of text to replace with their replacements.";
                example = {
                  "text to replace" = "replacement text";
                  "another text" = "another replacement";
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
              source = tomlFormat.generate "whispering-config" {
                audio = cfg.settings.audio;
                model = {
                  repo = cfg.settings.model.repo;
                  filename = cfg.settings.model.filename;
                  prompt = cfg.settings.model.prompt;
                  replacements = cfg.settings.model.replacements;
                };
                paths = cfg.settings.paths;
                shortcuts = cfg.settings.shortcuts;
              };
              mode = "0644";
            };

            # Add udev rules for input device access
            services.udev.extraRules = ''
              # Whispering udev rules
              # This file contains rules to allow the whispering user to access input devices

              # Allow whispering user to access /dev/uinput
              KERNEL=="uinput", GROUP="${cfg.group}", MODE="0660"

              # Allow whispering user to access input devices
              KERNEL=="event*", GROUP="${cfg.group}", MODE="0660"
            '';

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
                SupplementaryGroups = [
                  "${cfg.group}"
                ];
                # Display server specific environment variables
                Environment = displayEnv ++ (lib.mapAttrsToList (name: value: "${name}=${value}") cfg.environment);
                # Additional service configuration
              } // cfg.serviceConfig;
            };
          };
        };
    in
    {
      packages = forAllSystems (system: {
        default = buildPackage system;
        whispering = buildPackage system;
      });

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
                cmake
              ] ++ (if pkgs.stdenv.isDarwin then [ clang ] else [ ]);
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

      # Add NixOS module
      nixosModules = {
        default = nixosModule;
        whispering = nixosModule; # Add an alias for easier importing
      };

      # Add overlay to make the package available in nixpkgs
      overlays = {
        default = final: prev: {
          whispering = final.callPackage ./nix/package.nix { };
        };
        whispering = final: prev: {
          whispering = final.callPackage ./nix/package.nix { };
        };
      };
    };
}
