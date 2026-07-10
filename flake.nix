{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    treefmt-nix = {
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
    { self
    , nixpkgs
    , treefmt-nix
    , fenix
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      eachSystem = f: nixpkgs.lib.genAttrs systems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ overlays.default ];
          };
        in
        f pkgs);

      overlays.default = nixpkgs.lib.composeExtensions fenix.overlays.default (import ./nixos/overlays/horae);

      treefmtEval = eachSystem (pkgs:
        treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.lock";
          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
            taplo.enable = true;
            mdformat.enable = true;
          };
        });
    in
    {
      legacyPackages = eachSystem (pkgs: pkgs);

      packages = eachSystem (pkgs: {
        inherit (pkgs) horae;
        default = pkgs.horae;
      });

      devShells = eachSystem (pkgs:
        {
          default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self.checks.${pkgs.stdenv.buildPlatform.system};
            packages = (with pkgs; [
              dioxus-cli
              sqlx-cli
              postgresql
              wasm-pack # NOTE: wasm-bindgen version must match exactly
              nil
            ]);
          };
        }
      );

      formatter = eachSystem (pkgs: treefmtEval.${pkgs.stdenv.buildPlatform.system}.config.build.wrapper);

      checks = eachSystem (pkgs:
        {
          inherit (pkgs) horae;

          fmt = treefmtEval.${pkgs.stdenv.buildPlatform.system}.config.build.check self;

          e2e = pkgs.testers.nixosTest {
            name = "horae-e2e";
            nodes.server = { pkgs, ... }: {
              imports = [ self.nixosModules.default ];
              services.horae.enable = true;
              services.horae.database.createLocally = true;
              systemd.services.horae.environment.DEV_LOGIN = "1";
              # Put horae on PATH so the test script can call `horae seed`
              environment.systemPackages = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
            };
            testScript = ''
              server.start()
              server.wait_for_unit("postgresql.service")
              server.wait_for_unit("horae.service")
              server.wait_for_open_port(3000)

              # Health check
              server.succeed("curl -s http://localhost:3000/health | grep -q ok")

              # Seed data — run as the horae user (DynamicUser in systemd creates it)
              # so the unix socket auth matches the DB owner.
              server.succeed("sudo -u horae DATABASE_URL=postgres:///horae horae seed")

              # Dev login: POST returns 303 redirect — don't use -f (fails on non-2xx)
              status = server.succeed(
                "curl -s -o /dev/null -w '%{http_code}' -X POST http://localhost:3000/auth/dev-login"
              ).strip()
              assert status == "303", f"Expected 303 redirect, got: {status}"

              # Full login flow with cookie jar (follow redirect)
              server.succeed(
                "curl -s -c /tmp/cookies.txt -L -X POST http://localhost:3000/auth/dev-login -o /dev/null"
              )

              # Harvest API: list time entries (session-authenticated)
              result = server.succeed(
                "curl -s -b /tmp/cookies.txt http://localhost:3000/harvest/v2/time_entries"
              )
              assert '"time_entries"' in result, f"Expected Harvest envelope, got: {result[:200]}"
              assert '"per_page"' in result, f"Missing pagination field in: {result[:200]}"
            '';
          };
        }
      );

      nixosModules.default = {
        imports = [ ./nixos/modules/horae ];
        # TODO: Probably this isn't necessary
        _module.args.self = self;
      };

      apps = eachSystem (pkgs:
        {
          preview-site =
            let
              script = pkgs.writeShellApplication {
                name = "preview-site";
                runtimeInputs = [ pkgs.python3 pkgs.git ];
                text = ''
                  root=$(git rev-parse --show-toplevel)
                  echo "Serving site at http://localhost:8080"
                  exec python3 -m http.server 8080 --directory "$root/site"
                '';
              };
            in
            {
              type = "app";
              program = "${script}/bin/preview-site";
              meta.description = "Serves the static site locally on http://localhost:8080.";
            };

          qemu-vm =
            let
              guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] pkgs.stdenv.hostPlatform.system;
              debugConfig = nixpkgs.lib.nixosSystem {
                system = null;
                specialArgs = { inherit self; };
                modules = [
                  "${nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
                  self.nixosModules.default
                  ({
                    virtualisation.host.pkgs = pkgs;
                    nixpkgs.hostPlatform = guestSystem;
                  })
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
                        authentication = nixpkgs.lib.mkOverride 10 ''
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
