{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

    blueprint = {
      url = "github:numtide/blueprint";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };
  };

  outputs =
    inputs:
    let
      inherit (inputs.nixpkgs) lib;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      blueprint = inputs.blueprint {
        inherit inputs systems;
        prefix = "nix";
        # fenix provides the Rust toolchain used by nix/package.nix.
        nixpkgs.overlays = [ inputs.fenix.overlays.default ];
      };
    in
    blueprint
    // {
      # Developer convenience: `nix run .#qemu-vm` boots a NixOS VM running
      # Horae against a local PostgreSQL, with dev login enabled. Blueprint has
      # no `apps/` convention, so this is wired up here per system.
      apps = lib.genAttrs systems (system:
        (blueprint.apps.${system} or { }) // {
          qemu-vm =
            let
              pkgs = inputs.nixpkgs.legacyPackages.${system};
              guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
              debugConfig = lib.nixosSystem {
                system = null;
                modules = [
                  "${inputs.nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
                  inputs.self.nixosModules.horae
                  {
                    virtualisation.host.pkgs = pkgs;
                    nixpkgs.hostPlatform = guestSystem;
                  }
                  (
                    { pkgs, ... }:
                    {
                      services.horae = {
                        enable = true;
                        openFirewall = true;
                        database.createLocally = true;
                        # Dev login: skip OIDC, auto-login as admin.
                        secretKeyFile = null;
                      };
                      systemd.services.horae.environment.DEV_LOGIN = "1";

                      # Allow TCP connections from localhost (for host → guest Postgres).
                      services.postgresql = {
                        enableTCPIP = true;
                        # Grant CREATEDB so sqlx::test can create temp databases from the host.
                        initialScript = pkgs.writeText "grant-createdb.sql" ''
                          ALTER USER horae CREATEDB;
                        '';
                        authentication = lib.mkOverride 10 ''
                          # TYPE  DATABASE  USER    ADDRESS         METHOD
                          local   all       all                     trust
                          host    all       all     127.0.0.1/32    trust
                          host    all       all     ::1/128         trust
                          host    all       all     10.0.2.0/24     trust
                        '';
                      };

                      services.openssh = {
                        enable = true;
                        settings = {
                          PermitRootLogin = "yes";
                          PermitEmptyPasswords = "yes";
                        };
                      };
                      security.pam.services.sshd.allowNullPassword = true;
                      users.extraUsers.root.password = "";

                      virtualisation = {
                        forwardPorts = [
                          { from = "host"; host.port = 2222; guest.port = 22; }
                          { from = "host"; host.port = 3000; guest.port = 3000; }
                          { from = "host"; host.port = 5432; guest.port = 5432; }
                        ];
                      };

                      networking.firewall.allowedTCPPorts = [ 3000 5432 ];

                      environment.systemPackages = [ pkgs.postgresql ];

                      # TODO: We don't care about this, IIRC there's a way to suppress the warning
                      system.stateVersion = "25.05";
                    }
                  )
                ];
              };
            in
            {
              type = "app";
              program = "${debugConfig.config.system.build.vm}/bin/run-nixos-vm";
              meta.description = "Starts a NixOS VM with PostgreSQL and Horae.";
            };
        });
    };
}
