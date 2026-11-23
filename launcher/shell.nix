# Build environment for the launcher on NixOS:
# `nix-shell` in this folder, then `cargo build`.
{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ pkg-config clang mold ];
  buildInputs = with pkgs; [
    libxkbcommon wayland vulkan-loader fontconfig freetype alsa-lib openssl
    udev libx11 libxcb libxcursor libxrandr libxi
  ];
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [ libxkbcommon wayland vulkan-loader fontconfig freetype alsa-lib udev libx11 libxcb libxcursor libxrandr libxi ];
}
