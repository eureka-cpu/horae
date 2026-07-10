{ lib
, mkPackagesFor
, fenix
,
}:
# Rebuilds horae against the consumer's `final`, so deps are shared with the
# rest of their system and cross-compilation works (pkgsCross.<target>.horae).
# Composes fenix because nix/package.nix pulls the Rust toolchain from
# `pkgs.fenix`. Trade-off vs overlays.default: cache only hits when the
# consumer's nixpkgs revision matches ours.
lib.composeExtensions fenix (
  final: _prev: {
    horae = (mkPackagesFor final).default;
  }
)
