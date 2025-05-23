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

      # Function to get system-specific build inputs
      getBuildInputs =
        pkgs: system:
        if pkgs.stdenv.isDarwin then
          with pkgs;
          [
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
            cudaPackages.libcufft
          ];

      # Function to get system-specific environment variables
      getEnvVars =
        pkgs: system: onnxruntime:
        if pkgs.stdenv.isDarwin then
          {
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = ''-I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"'';
            CC = "${pkgs.clang}/bin/clang";
            CXX = "${pkgs.clang}/bin/clang++";
            MACOSX_DEPLOYMENT_TARGET = "11.0";
            CFLAGS = "-fmodules";
            LIBRARY_PATH = "${pkgs.darwin.libiconv}/lib";
            ORT_LIB_LOCATION = "${onnxruntime.metal}/lib";
          }
        else
          {
            # LD_LIBRARY_PATH = "${pkgs.llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib:${pkgs.cudaPackages.cudatoolkit}/lib:${pkgs.cudaPackages.cudnn.lib}/lib";
            LD_LIBRARY_PATH = "/run/opengl-driver/lib:${pkgs.cudaPackages.cudatoolkit}/lib:${pkgs.cudaPackages.cudnn.lib}/lib";
            ORT_LIB_LOCATION = "${onnxruntime.gpu}/lib";
          };

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
          darwinInputs =
            if pkgs.stdenv.isDarwin then
              {
                inherit (pkgs) ;
              }
            else
              { };
          pkg = pkgs.callPackage ./nix/package.nix (
            {
              inherit (pkgs) libnotify dbus libiconv;
            }
            // darwinInputs
          );
        in
        pkg;

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
          usingWayland = config.programs.sway.enable or false || config.programs.hyprland.enable or false;
          usingX11 = config.services.xserver.enable or false;
          # Detect display server
          isWayland = usingWayland;
          # Get display server specific environment variables
          pkg = (buildPackage pkgs.system);
          displayEnv =
            if isWayland then
              [
                "XDG_RUNTIME_DIR=/run/user/1000"
                "WAYLAND_DISPLAY=wayland-1"
                "LD_LIBRARY_PATH=/run/opengl-driver/lib"
              ]
            else
              [
                "DISPLAY=:0"
                "XAUTHORITY=/home/${cfg.user}/.Xauthority"
                "LD_LIBRARY_PATH=/run/opengl-driver/lib"
              ];
          # Create TOML format
          tomlFormat = pkgs.formats.toml { };
        in
        {
          # assertions = [
          #   {
          #     assertion = usingWayland || usingX11;
          #     message = "Either Wayland or X11 must be enabled.";
          #   }
          # ];
          options.services.whispering = {
            enable = lib.mkEnableOption "Whispering service";

            package = lib.mkOption {
              type = lib.types.package;
              default = if usingX11 then pkg.linux-x11 else pkg.default;
              description = "The Whispering package to use.";
            };

            user = lib.mkOption {
              type = lib.types.str;
              description = "User account under which Whispering runs.";
            };

            group = lib.mkOption {
              type = lib.types.str;
              default = "users";
              description = "Group under which Whispering runs.";
            };

            dataDir = lib.mkOption {
              type = lib.types.path;
              default = "/home/${cfg.user}";
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
                sample_format = lib.mkOption {
                  type = lib.types.enum [
                    "f32"
                    "i16"
                  ];
                  default = "f32";
                  description = "Sample format (f32 or i16).";
                };
                device = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Audio input device name (e.g., 'sysdefault:CARD=C920'). If not specified, the default device will be used.";
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
                  default = "${cfg.dataDir}/.cache/whispering";
                  description = "Cache directory for storing temporary files.";
                };
                recording_path = lib.mkOption {
                  type = lib.types.path;
                  default = "${cfg.dataDir}/.cache/whispering/recorded.wav";
                  description = "Path to the recorded audio file.";
                };
              };

              # Activation settings
              activation = {
                trigger = lib.mkOption {
                  type = lib.types.attrsOf lib.types.anything;
                  default = {
                    type = "push_to_talk";
                  };
                  description = "Type of activation to use for recording control.";
                  example = {
                    type = "toggle_vad";
                    threshold = 0.5;
                    silence_duration = 1.0;
                    speech_duration = 0.3;
                    pre_buffer_duration = 1.0;
                  };
                };
                notify = lib.mkOption {
                  type = lib.types.bool;
                  default = true;
                  description = "Displays a notification about the capturing.";
                };
                autosend = lib.mkOption {
                  type = lib.types.bool;
                  default = false;
                  description = "Automatically hit enter after sending the text.";
                };
                keys = lib.mkOption {
                  type = lib.types.listOf lib.types.str;
                  default = [
                    "ControlLeft"
                    "Space"
                  ];
                  description = "Keys that need to be pressed in sequence to start recording.";
                };
              };
            };

            # Option to disable notifications completely
            disableNotifications = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Disable notifications completely. Useful if you don't have a notification daemon running.";
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
            users.groups = lib.optionalAttrs (cfg.group == "whispering") {
              whispering = { };
            };

            users.users = lib.optionalAttrs (cfg.user == "whispering") {
              whispering = {
                isSystemUser = true;
                group = cfg.group;
                home = cfg.dataDir;
                createHome = true;
                extraGroups = [
                  "audio"
                  "input"
                  "messagebus"
                  "video"
                ];
              };
            };

            # Create configuration file
            environment.etc."whispering/config.toml" = {
              source = tomlFormat.generate "whispering-config" {
                audio =
                  if cfg.settings.audio.device == null then
                    builtins.removeAttrs cfg.settings.audio [ "device" ]
                  else
                    cfg.settings.audio;
                model = {
                  repo = cfg.settings.model.repo;
                  filename = cfg.settings.model.filename;
                  prompt = cfg.settings.model.prompt;
                  replacements = cfg.settings.model.replacements;
                };
                paths = cfg.settings.paths;
                activation = cfg.settings.activation;
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

            # Ensure proper permissions for input devices and clipboard
            system.activationScripts.whisperingPermissions = lib.mkIf cfg.enable {
              deps = [
                "users"
                "groups"
              ];
              text = ''
                # Set ACL for uinput device
                if [ -e /dev/uinput ]; then
                  ${pkgs.acl}/bin/setfacl -m u:${cfg.user}:rw- /dev/uinput
                fi

                # Set ACL for ALSA devices
                if [ -e /dev/snd ]; then
                  ${pkgs.acl}/bin/setfacl -m u:${cfg.user}:rw- /dev/snd/*
                fi
              '';
            };

            systemd.services.whispering = {
              description = "Whispering service";
              wantedBy = [ "multi-user.target" ];
              after = [
                "network.target"
                "systemd-udev-settle.service"
                "sound.target"
                "dbus.service"
              ];
              requires = [
                "sound.target"
                "dbus.service"
              ];
              serviceConfig = {
                Type = "simple";
                User = cfg.user;
                Group = cfg.group;
                WorkingDirectory = cfg.dataDir;
                RuntimeDirectory = "whispering";
                RuntimeDirectoryMode = "0755";
                ExecStart = "${cfg.package}/bin/whispering --config /etc/whispering/config.toml";
                Restart = "on-failure";
                RestartSec = "10s";
                # Required for CUDA, audio and input devices
                SupplementaryGroups = [
                  "${cfg.group}"
                  "audio"
                  "input"
                  "messagebus"
                  "video"
                ];
                # Display server specific environment variables
                Environment =
                  displayEnv
                  ++ [
                    "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus"
                    "XDG_RUNTIME_DIR=/run/user/1000"
                  ]
                  ++ (lib.mapAttrsToList (name: value: "${name}=${value}") cfg.environment);
                # Additional service configuration
              } // cfg.serviceConfig;
            };
          };
        };
    in
    {
      packages = forAllSystems (system: {
        default = (buildPackage system).default;
        whisper-darwin = (buildPackage system).whisper-darwin;
        linux-wayland = (buildPackage system).linux-wayland;
        linux-x11 = (buildPackage system).linux-x11;
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
          # Fetch different versions of Onyx runtime libraries
          onnxruntime = {
            gpu = pkgs.fetchzip {
              url = "https://parcel.pyke.io/v2/delivery/ortrs/packages/msort-binary/1.20.0/ortrs_dylib_cu12-v1.20.0-x86_64-unknown-linux-gnu.tgz";
              sha256 = "sha256-7QzVQGpea9FSb7OEEYwfOF6qI6+rt+uCFytH23GKQMU=";
            };
            metal = pkgs.fetchzip {
              url = "https://parcel.pyke.io/v2/delivery/ortrs/packages/msort-binary/1.20.0/ortrs_static-v1.20.0-aarch64-apple-darwin.tgz";
              sha256 = "sha256-cmiVcY7ds+WcodwbKnOnPsWDGoGE0+iSdEAy1erMUso=";
            };
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
                libxkbcommon
              ] ++ getBuildInputs pkgs system;
              RUST_LOG = "whispering=info";
            }
            // (getEnvVars pkgs system onnxruntime)
          );
        }
      );

      # Add NixOS module
      nixosModules = {
        default = nixosModule;
        whispering = nixosModule; # Add an alias for easier importing
      };

      nixConfig = {
        extra-subsituters = [
          "https://narsil.cachix.org"
        ];
        extra-trusted-public-keys = [
          "narsil.cachix.org-1:eTLhYqg5uVi7Pv3x6L/Ov8NdESEpGeViXiwGKLYpo90="
        ];
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
