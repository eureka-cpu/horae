{ perSystem, pkgs, ... }:
perSystem.self.default.overrideAttrs (old: {
  pname = "horae-nextest";
  nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.cargo-nextest ];

  # Let compile-time sqlx macros (query!, query_as!, …) resolve from the
  # .sqlx/ cache instead of requiring a live database connection.
  SQLX_OFFLINE = "true";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    # horae-core: pure domain tests — no database required.
    cargo nextest run -p horae-core
    # horae #[sqlx::test] integration tests need a live Postgres and are
    # covered by the NixOS e2e check instead.
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
