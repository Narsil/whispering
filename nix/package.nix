# This file defines the package with its variants
{ lib
, stdenv
, rustPlatform
, pkg-config
, cmake
, llvmPackages
, openssl
, darwin
, udev
, libinput
, alsa-lib
, alsa-utils
, wayland
, wayland-protocols
, wayland-scanner
, xorg
, cudaPackages
}:

let
  # Common build inputs for all platforms
  commonBuildInputs = [
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
    ];
    envVars = {
      LD_LIBRARY_PATH = "${llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib:${cudaPackages.cudatoolkit}/lib";
      CUDA_PATH = "${cudaPackages.cudatoolkit}";
      EXTRA_LDFLAGS = "-L${cudaPackages.cudatoolkit}/lib/stubs";
      CUDA_TOOLKIT_ROOT_DIR = "${cudaPackages.cudatoolkit}";
      CMAKE_CUDA_COMPILER = "${cudaPackages.cuda_nvcc}/bin/nvcc";
    };
    cmakeArgs = "-DCMAKE_CUDA_COMPILER=${cudaPackages.cuda_nvcc}/bin/nvcc -DCMAKE_CUDA_ARCHITECTURES=all -DCUDA_TOOLKIT_ROOT_DIR=${cudaPackages.cudatoolkit} -DCUDA_INCLUDE_DIRS=${cudaPackages.cudatoolkit}/include -DCUDA_CUDART_LIBRARY=${cudaPackages.cuda_cudart}/lib/libcudart.so -DCUDA_NVCC_EXECUTABLE=${cudaPackages.cuda_nvcc}/bin/nvcc";
  };

  # Base derivation function
  mkWhispering = { features ? [], extraBuildInputs ? [], extraEnvVars ? {}, cmakeArgs ? "" }:
    rustPlatform.buildRustPackage.override { inherit stdenv; } rec {
      pname = "whispering";
      version = "0.1.0";
      src = ../.;

      cargoLock = {
        lockFile = ../Cargo.lock;
        outputHashes = {
          "rdev-0.5.3" = "sha256-Ws+690+zVIp+niZ7zgbCMSKPXjioiWuvCw30faOyIrA=";
          "whisper-rs-0.14.2" = "sha256-V+1RYWTVLHgPhRg11pz08jb3zqFtzv3ODJ1E+tf/Z9I=";
        };
      };

      buildFeatures = features;
      CMAKE_ARGS = cmakeArgs;
      nativeBuildInputs = commonBuildInputs;
      buildInputs = commonBuildInputs ++ extraBuildInputs;

      env = commonEnvVars // extraEnvVars;
    };

in
{
  # Darwin variant with Metal support
  darwin = mkWhispering {
    features = [ "metal" ];
    extraBuildInputs = with darwin; [
      apple_sdk.frameworks.CoreAudio
      apple_sdk.frameworks.AudioToolbox
      apple_sdk.frameworks.Metal
      apple_sdk.frameworks.MetalKit
      apple_sdk.frameworks.MetalPerformanceShaders
      apple_sdk.frameworks.Foundation
      apple_sdk.frameworks.AppKit
      apple_sdk.frameworks.UserNotifications
      libiconv
      openssl
    ];
    extraEnvVars = {
      CC = "${stdenv.cc}/bin/clang";
      CXX = "${stdenv.cc}/bin/clang++";
      MACOSX_DEPLOYMENT_TARGET = "11.0";
      CFLAGS = "-fmodules";
      OBJC_INCLUDE_PATH = "${darwin.apple_sdk.frameworks.Foundation}/include:${darwin.apple_sdk.frameworks.AppKit}/include";
      LIBRARY_PATH = "${darwin.libiconv}/lib";
    };
  };

  # Linux Wayland variant with CUDA support
  linux-wayland = mkWhispering {
    features = [ "wayland" "cuda" ];
    extraBuildInputs = [
      udev
      libinput
      alsa-lib
      alsa-utils
      openssl
      wayland
      wayland-protocols
      wayland-scanner
    ] ++ cudaConfig.buildInputs;
    extraEnvVars = cudaConfig.envVars;
    cmakeArgs = cudaConfig.cmakeArgs;
  };

  # Linux X11 variant with CUDA support
  linux-x11 = mkWhispering {
    features = [ "x11" "cuda" ];
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
    ] ++ cudaConfig.buildInputs;
    extraEnvVars = cudaConfig.envVars;
    cmakeArgs = cudaConfig.cmakeArgs;
  };
} 