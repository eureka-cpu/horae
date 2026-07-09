{ flake, ... }:
{ config
, lib
, pkgs
, ...
}:

let
  cfg = config.services.horae;
in
{
  options.services.horae = {
    enable = lib.mkEnableOption "Horae time tracking server";

    package = lib.mkOption {
      type = lib.types.package;
      default = flake.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "horae.packages.\${pkgs.stdenv.hostPlatform.system}.default";
      description = "The horae package to use.";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "The host address the Horae server listens on.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "The TCP port the Horae server listens on.";
    };

    database = {
      url = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = ''
          PostgreSQL connection URL. When null and database.createLocally is true,
          defaults to a local Unix-socket connection: postgres:///horae.
        '';
      };

      createLocally = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          When true and database.url is null, configure a local PostgreSQL
          instance with a `horae` database owned by the `horae` service user.
        '';
      };
    };

    secretKeyFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Path to a file containing environment variables with secrets
        (e.g. SESSION_SECRET, OIDC_CLIENT_SECRET). Loaded via systemd EnvironmentFile.
      '';
    };

    logLevel = lib.mkOption {
      # TODO: Should be an enum
      type = lib.types.str;
      default = "info";
      description = "Log verbosity level passed as HORAE_LOG.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to open the firewall for the Horae port.";
    };
  };

  config = lib.mkIf cfg.enable {
    # Local PostgreSQL instance managed by this module.
    services.postgresql = lib.mkIf cfg.database.createLocally {
      enable = true;
      ensureDatabases = [ "horae" ];
      ensureUsers = [
        {
          name = "horae";
          ensureDBOwnership = true;
        }
      ];
    };

    systemd.services.horae = {
      description = "Horae time tracking server";
      after = [ "network.target" ] ++ lib.optionals cfg.database.createLocally [ "postgresql.service" ];
      wants = lib.optionals cfg.database.createLocally [ "postgresql.service" ];
      wantedBy = [ "multi-user.target" ];

      environment = {
        HORAE_LOG = cfg.logLevel;
        DATABASE_URL =
          if cfg.database.url != null then
            cfg.database.url
          else if cfg.database.createLocally then
            "postgres:///horae"
          else
            "postgres://localhost/horae";
      };

      serviceConfig =
        {
          ExecStartPre = "${cfg.package}/bin/horae migrate run";
          ExecStart = "${cfg.package}/bin/horae serve --host ${cfg.host} --port ${toString cfg.port}";
          DynamicUser = true;
          StateDirectory = "horae";
          Restart = "on-failure";

          # Hardening
          NoNewPrivileges = true;
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
        }
        // lib.optionalAttrs (cfg.secretKeyFile != null) {
          EnvironmentFile = cfg.secretKeyFile;
        };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
