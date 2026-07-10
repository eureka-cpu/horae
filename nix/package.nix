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
  pname = "horae";
  version = "0.1.0";
  src = lib.cleanSourceWith {
    src = lib.cleanSource ../.;
    filter = path: _type: !(lib.hasSuffix ".nix" path) && !(lib.hasSuffix ".md" path);
  };
  cargoLock.lockFile = finalAttrs.src + "/Cargo.lock";
  doCheck = false;

  nativeBuildInputs = with buildPkgs; [
    dioxus-cli
    wasm-bindgen-cli_0_2_126 # Must match the version of wasm-bindgen in Cargo.toml
    wasm-pack
    binaryen
  ] ++ lib.optionals stdenv.hostPlatform.isDarwin [
    buildPkgs.darwin.sigtool
  ];

  # Use dx build — the same command as development — to compile both the
  # server binary and the WASM client bundle in one step.  cargoSetupHook
  # (from buildRustPackage) runs before this and populates $CARGO_HOME with
  # the vendored deps, so the cargo invocations inside dx work offline.
  # dx must run from crates/horae/ where Dioxus.toml lives.
  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    (cd crates/horae && dx build --release)
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    # dx build puts fullstack output under target/dx/{app}/{profile}/web/:
    #   server  — the server binary
    #   public/ — content-addressed WASM, JS, CSS assets + index.html
    local dxdir=target/dx/horae/release/web
    cp "$dxdir/server" $out/bin/horae
    cp -r "$dxdir/public" $out/bin/public
    runHook postInstall
  '';

  meta = {
    description = "A self-hostable time tracking server";
    mainProgram = "horae";
  };
})
