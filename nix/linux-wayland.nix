{ nixpkgs, common }:
pkgs:
let
  buildInputs = with pkgs; [
    udev
    libinput
    alsa-lib
    alsa-utils
    openssl
    wayland
    wayland-protocols
    wayland-scanner
    cudaPackages.cudatoolkit
    cudaPackages.cuda_cudart
    cudaPackages.cuda_nvcc
  ];

  envVars = {
    LD_LIBRARY_PATH = "${pkgs.llvmPackages.libclang.lib}/lib:/run/opengl-driver/lib:${pkgs.cudaPackages.cudatoolkit}/lib";
    CUDA_PATH = "${pkgs.cudaPackages.cudatoolkit}";
    EXTRA_LDFLAGS = "-L${pkgs.cudaPackages.cudatoolkit}/lib/stubs";
    CUDA_TOOLKIT_ROOT_DIR = "${pkgs.cudaPackages.cudatoolkit}";
    CMAKE_CUDA_COMPILER = "${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
  };

  features = [ "wayland" "cuda" ];

  cmakeArgs = "-DCMAKE_CUDA_COMPILER=${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc -DCMAKE_CUDA_ARCHITECTURES=all -DCUDA_TOOLKIT_ROOT_DIR=${pkgs.cudaPackages.cudatoolkit} -DCUDA_INCLUDE_DIRS=${pkgs.cudaPackages.cudatoolkit}/include -DCUDA_CUDART_LIBRARY=${pkgs.cudaPackages.cuda_cudart}/lib/libcudart.so -DCUDA_NVCC_EXECUTABLE=${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
in
{
  inherit buildInputs envVars features cmakeArgs;
} 