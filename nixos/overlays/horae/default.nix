final: prev:
let
  inherit (prev) lib stdenv;

  isCross = with stdenv; buildPlatform != hostPlatform;
  # Use the build machine's packages if cross compiling
  pkgs = if isCross then prev.pkgsBuildBuild else prev;
  toolchain = with pkgs.fenix; combine ([
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
    targets.${with stdenv.hostPlatform.rust; builtins.trace "Adding component rust-std-${rustcTarget}-stable" rustcTarget}.stable.rust-std);
  rustPlatform = pkgs.makeRustPlatform {
    cargo = toolchain;
    rustc = toolchain;
  };
in
{
  horae = rustPlatform.buildRustPackage (finalAttrs: {
    # TODO: Get metadata from Cargo.toml
    pname = "horae";
    version = "0.1.0";
    src = lib.cleanSourceWith {
      src = lib.cleanSource ../../../.;
      filter = path: _type: !(lib.hasSuffix ".nix" path) && !(lib.hasSuffix ".md" path);
    };
    cargoLock.lockFile = finalAttrs.src + "/Cargo.lock";
    # TODO: Build the all programs
    buildFeatures = [ "server" ];
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
  });
}
