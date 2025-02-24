with import <nixpkgs> {};
stdenv.mkDerivation {
  name = "env";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    rustc
    cargo
    rustfmt
    wayland
    glfw-wayland
    libxkbcommon
  ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    wayland
    glfw-wayland
    libxkbcommon
  ]);
}
