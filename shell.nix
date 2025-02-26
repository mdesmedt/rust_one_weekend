with import <nixpkgs> {};
stdenv.mkDerivation {
  name = "env";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    # Rust packages
    rustc
    cargo
    rustfmt
    # Project packages
    wayland
    glfw-wayland
    libxkbcommon
  ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    wayland
    glfw-wayland
    libxkbcommon
  ]);
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
