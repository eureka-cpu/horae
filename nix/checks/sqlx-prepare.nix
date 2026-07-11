{ perSystem, pkgs, ... }:
perSystem.self.default.overrideAttrs (old: {
  pname = "horae-sqlx-prepare";
  nativeBuildInputs = old.nativeBuildInputs ++ [
    pkgs.postgresql
    pkgs.sqlx-cli
  ];

  # Override SQLX_OFFLINE so macros connect to our temporary database.
  SQLX_OFFLINE = "";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)

    # Start a temporary PostgreSQL instance (unix socket only, no TCP).
    export PGDATA=$(mktemp -d)
    initdb -D "$PGDATA" --no-locale --encoding=UTF8 -U postgres
    pg_ctl -D "$PGDATA" -l "$PGDATA/log" \
      -o "--unix_socket_directories=$PGDATA --listen_addresses=" start
    createdb -h "$PGDATA" -U postgres horae
    export DATABASE_URL="postgres://postgres@localhost/horae?host=$PGDATA"

    # Apply migrations then verify the .sqlx/ cache is up-to-date.
    cargo sqlx migrate run --source crates/horae/migrations
    cargo sqlx prepare --workspace --check -- --features server

    pg_ctl -D "$PGDATA" stop
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
