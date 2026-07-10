{ pkgs, ... }:
let
  inherit (pkgs) lib stdenv;

  isCross = stdenv.buildPlatform != stdenv.hostPlatform;
  # Use the build machine's packages if cross compiling.
  buildPkgs = if isCross then pkgs.pkgsBuildBuild else pkgs;
  toolchain = with buildPkgs.fenix; combine ([
    (stable.withComponents [
      "rustc"
      "cargo"
      "clippy"
      "rust-std"
      "rust-analyzer"
      "rust-src"
    ])
    targets.wasm32-unknown-unknown.stable.rust-std
  ] ++ lib.optional isCross
    targets.${stdenv.hostPlatform.rust.rustcTarget}.stable.rust-std);
  rustPlatform = buildPkgs.makeRustPlatform {
    cargo = toolchain;
    rustc = toolchain;
  };
in
rustPlatform.buildRustPackage (finalAttrs: {
  # TODO: Get metadata from Cargo.toml
  pname = "horae";
  version = "0.1.0";
  src = lib.cleanSourceWith {
    src = lib.cleanSource ../.;
    filter = path: _type: !(lib.hasSuffix ".nix" path) && !(lib.hasSuffix ".md" path);
  };
  cargoLock.lockFile = finalAttrs.src + "/Cargo.lock";
  # TODO: Build the all programs
  buildFeatures = [ "server" ];
  # The workspace root is virtual, so select the app crate explicitly
  # (`cargo build --features` is not allowed at a virtual-workspace root).
  cargoBuildFlags = [ "-p" "horae" ];
  # TODO: Add check derivations instead
  doCheck = false;
  # dioxus-server expects a public/ dir next to the binary for static
  # assets. Until we integrate `dx build` into the Nix build, create
  # an empty one so the server starts without panicking.
  postInstall = ''
    mkdir -p $out/bin/public
  '';
  meta = {
    description = "A self-hostable time tracking server";
    mainProgram = "horae";
  };
})
