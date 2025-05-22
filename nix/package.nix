# This file defines the package with its variants
{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  cmake,
  llvmPackages,
  openssl,
  # onnxruntime,
  udev,
  libinput,
  alsa-lib,
  alsa-utils,
  wayland,
  wayland-protocols,
  wayland-scanner,
  libxkbcommon,
  xorg,
  cudaPackages,
  libnotify,
  dbus,
  libiconv,
}:

let
  # Fetch different versions of Onyx runtime libraries
  onnxruntime = {
    gpu = builtins.fetchTarball {
      url = "https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/onnxruntime-linux-x64-gpu-1.22.0.tgz";
      sha256 = "sha256:189hcwbr2irxy512lhj22ff9w3pwqhbvhja3xrk11vqk4rxq1sxh";
    };
    metal = builtins.fetchTarball {
      url = "https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/onnxruntime-osx-arm64-1.22.0.tgz";
      sha256 = "sha256-0000000000000000000000000000000000000000000000000000";
    };
  };

  filteredSrc = lib.cleanSourceWith {
    src = ../.;
    filter =
      path: type:
      let
        rel = builtins.baseNameOf path;
        dir = builtins.dirOf path;
        # Keep essential Rust project files and directories
      in
      (rel == "Cargo.toml")
      || (rel == "config.example.toml")
      || (rel == "Cargo.lock")
      || (rel == "rust-toolchain.toml")
      || (rel == "rust-toolchain")
      || (lib.hasPrefix (toString ../src) path)
      || (lib.hasPrefix (toString ../benches) path)
      || (lib.hasPrefix (toString ../examples) path)
      || (lib.hasPrefix (toString ../tests) path)
      || (lib.hasPrefix (toString ../build.rs) path)
      || (rel == "README.md")
      || (rel == "LICENSE");
  };
  # Common build inputs for all platforms
  commonBuildInputs = [
    openssl
  ];
  commonNativeBuildInputs = [
    llvmPackages.libclang
    pkg-config
    cmake
  ];

  # Common environment variables
  commonEnvVars = {
    LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = ''-I"${llvmPackages.libclang.lib}/lib/clang/${llvmPackages.libclang.version}/include"'';
  };

  # CUDA-specific configuration
  cudaConfig = {
    buildInputs = with cudaPackages; [
      cudatoolkit
      cuda_cudart
      cuda_nvcc
      onnxruntime.gpu
    ];
    envVars = {
      LD_LIBRARY_PATH = "${llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib:${cudaPackages.cudatoolkit}/lib";
      CUDA_PATH = "${cudaPackages.cudatoolkit}";
      EXTRA_LDFLAGS = "-L${cudaPackages.cudatoolkit}/lib/stubs";
      CUDA_TOOLKIT_ROOT_DIR = "${cudaPackages.cudatoolkit}";
      CMAKE_CUDA_COMPILER = "${cudaPackages.cuda_nvcc}/bin/nvcc";
      ORT_LIB_LOCATION = "${onnxruntime.gpu}/lib";
    };
    cmakeArgs = "-DCMAKE_CUDA_COMPILER=${cudaPackages.cuda_nvcc}/bin/nvcc -DCMAKE_CUDA_ARCHITECTURES=all -DCUDA_TOOLKIT_ROOT_DIR=${cudaPackages.cudatoolkit} -DCUDA_INCLUDE_DIRS=${cudaPackages.cudatoolkit}/include -DCUDA_CUDART_LIBRARY=${cudaPackages.cuda_cudart}/lib/libcudart.so -DCUDA_NVCC_EXECUTABLE=${cudaPackages.cuda_nvcc}/bin/nvcc";
  };

  # Base derivation function
  mkWhispering =
    {
      features ? [ ],
      extraBuildInputs ? [ ],
      extraNativeBuildInputs ? [ ],
      extraEnvVars ? { },
      extraPreConfigure ? "",
      cmakeArgs ? "",
    }:
    rustPlatform.buildRustPackage.override { inherit stdenv; } rec {
      pname = "whispering";
      version = "0.1.0";
      src = filteredSrc;

      cargoLock = {
        lockFile = ../Cargo.lock;
        outputHashes = {
          "rdev-0.6.0" = "sha256-T4yQXAzW52xBCZAjtKnarMKrXB6wybe/omjozcm1JjU=";
          "whisper-rs-0.14.2" = "sha256-V+1RYWTVLHgPhRg11pz08jb3zqFtzv3ODJ1E+tf/Z9I=";
        };
      };
      preConfigure = extraPreConfigure;

      buildFeatures = features;
      CMAKE_ARGS = cmakeArgs;
      nativeBuildInputs = commonNativeBuildInputs ++ extraNativeBuildInputs;
      buildInputs = commonBuildInputs ++ extraBuildInputs;

      env = commonEnvVars // extraEnvVars;
    };

in
rec {
  # Default package based on platform
  default = if stdenv.isDarwin then whispering-darwin else linux-wayland;

  # Darwin variant with Metal support
  whispering-darwin = mkWhispering {
    features = [ "metal" ];
    extraNativeBuildInputs = [
      rustPlatform.bindgenHook
    ];
    extraBuildInputs = [
      libiconv
      openssl
      onnxruntime.metal
    ];
    extraPreConfigure = ''
      echo $NIX_CFLAGS_COMPILE
      export NIX_CFLAGS_COMPILE="$NIX_CFLAGS_COMPILE -march=armv8.6-a";
      export NIX_CXXFLAGS_COMPILE="$NIX_CXXFLAGS_COMPILE -march=armv8.6-a";
      echo $NIX_CFLAGS_COMPILE
      export ORT_LIB_LOCATION = "${onnxruntime.metal}/lib";
    '';
  };

  # Linux Wayland variant with CUDA support
  linux-wayland = mkWhispering {
    features = [
      "wayland"
      "cuda"
    ];
    extraBuildInputs = [
      udev
      libinput
      alsa-lib
      alsa-utils
      openssl
      wayland
      wayland-protocols
      wayland-scanner
      libnotify
      libxkbcommon
      dbus
    ] ++ cudaConfig.buildInputs;
    extraEnvVars = cudaConfig.envVars;
    cmakeArgs = cudaConfig.cmakeArgs;
  };

  # Linux X11 variant with CUDA support
  linux-x11 = mkWhispering {
    features = [
      "x11"
      "cuda"
    ];
    extraBuildInputs = [
      udev
      libinput
      alsa-lib
      alsa-utils
      openssl
      xorg.libX11
      xorg.libXcursor
      xorg.libXrandr
      xorg.libXi
      xorg.libXtst
      libnotify
      dbus
    ] ++ cudaConfig.buildInputs;
    extraEnvVars = cudaConfig.envVars;
    cmakeArgs = cudaConfig.cmakeArgs;
  };
}
