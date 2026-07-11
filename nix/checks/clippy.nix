{ perSystem, ... }:
perSystem.self.default.overrideAttrs (_: {
  pname = "horae-clippy";

  # Let compile-time sqlx macros (query!, query_as!, …) resolve from the
  # .sqlx/ cache instead of requiring a live database connection.
  SQLX_OFFLINE = "true";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    cargo clippy -p horae-core --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
