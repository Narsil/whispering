{ nixpkgs, common }:
pkgs:
let
  buildInputs = with pkgs; [
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
  ];

  envVars = {
    CC = "${pkgs.clang}/bin/clang";
    CXX = "${pkgs.clang}/bin/clang++";
    MACOSX_DEPLOYMENT_TARGET = "11.0";
    CFLAGS = "-fmodules";
    OBJC_INCLUDE_PATH = "${pkgs.darwin.apple_sdk.frameworks.Foundation}/include:${pkgs.darwin.apple_sdk.frameworks.AppKit}/include";
    LIBRARY_PATH = "${pkgs.darwin.libiconv}/lib";
  };

  features = [ "metal" ];
in
{
  inherit buildInputs envVars features;
} 