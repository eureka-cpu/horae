{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    fenix.inputs.rust-analyzer-src.follows = "";
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

      forAllSystems = nixpkgs.lib.genAttrs systems;

      treefmtEval = forAllSystems (
        system:
        treefmt-nix.lib.evalModule nixpkgs.legacyPackages.${system} {
          projectRootFile = "flake.nix";
          programs.nixpkgs-fmt.enable = true;
          programs.rustfmt.enable = true;
          programs.taplo.enable = true;
          programs.mdformat.enable = true;
        }
      );
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          toolchain = fenix.packages.${system}.stable.withComponents [
            "rustc"
            "cargo"
            "rust-std"
          ];
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "horae";
            version = "0.1.0";
            # TODO: Exclude extraneous files from source
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
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
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          fenixPkgs = fenix.packages.${system};
          # TODO: Fix this combine so that we don't need to specify RUST_SRC_PATH
          stableToolchain = fenix.packages.${system}.combine [
            fenixPkgs.stable.rustc
            fenixPkgs.stable.cargo
            fenixPkgs.stable.clippy
            fenixPkgs.stable.rust-src
            fenixPkgs.stable.rust-analyzer
            fenixPkgs.targets.wasm32-unknown-unknown.stable.rust-std
          ];
        in
        {
          default = pkgs.mkShell {
            packages = [
              stableToolchain
              pkgs.dioxus-cli
              pkgs.sqlx-cli
              pkgs.postgresql
              pkgs.wasm-pack
              pkgs.nil
            ];
            RUST_SRC_PATH = "${fenixPkgs.stable.rust-src}/lib/rustlib/src/rust/library";
          };
        }
      );

      formatter = forAllSystems (system: treefmtEval.${system}.config.build.wrapper);

      checks = forAllSystems (system:
        let
          testPkgs = nixpkgs.legacyPackages.${system};
        in
        {
          fmt = treefmtEval.${system}.config.build.check self;

          # NixOS e2e test: golden path from boot to Harvest API query.
          # Run with: nix build .#checks.<system>.e2e
          e2e = testPkgs.testers.nixosTest {
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
        imports = [ ./nixos/modules/horae/default.nix ];
        _module.args.self = self;
      };

      # Production NixOS configuration (no QEMU, no port forwards).
      nixosConfigurations = {
        default = nixpkgs.lib.nixosSystem {
          specialArgs = { inherit self; };
          modules = [
            self.nixosModules.default
            ({ nixpkgs.hostPlatform = "x86_64-linux"; })
            (
              { ... }:
              {
                services.horae.enable = true;
                services.horae.database.createLocally = true;

                # Minimal bootable config
                boot.loader.grub.device = "nodev";
                fileSystems."/" = {
                  device = "none";
                  fsType = "tmpfs";
                  options = [ "mode=0755" ];
                };
                system.stateVersion = "25.05";
              }
            )
          ];
        };
      };

      # `nix run .#dev-vm` starts the dev NixOS VM.
      apps = forAllSystems (system:
        {
          dev-vm =
            let
              # Dev VM: NixOS with Postgres, port-forwarded to localhost.
              # `virtualisation.host.pkgs` makes the run script and QEMU come from
              # the host (e.g. aarch64-darwin) while the guest boots aarch64-linux.
              hostPkgs = nixpkgs.legacyPackages.${system};
              guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
              debugConfig = nixpkgs.lib.nixosSystem {
                system = null;
                specialArgs = { inherit self; };
                modules = [
                  "${nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
                  self.nixosModules.default
                  ({
                    virtualisation.host.pkgs = hostPkgs;
                    nixpkgs.hostPlatform = guestSystem;
                  })
                  (
                    { pkgs, ... }:
                    {
                      services.horae.enable = true;
                      services.horae.database.createLocally = true;

                      # Allow TCP connections from localhost (for host → guest Postgres).
                      services.postgresql.enableTCPIP = true;
                      services.postgresql.authentication = nixpkgs.lib.mkOverride 10 ''
                        # TYPE  DATABASE  USER    ADDRESS         METHOD
                        local   all       all                     trust
                        host    all       all     127.0.0.1/32    trust
                        host    all       all     ::1/128         trust
                        host    all       all     10.0.2.0/24     trust
                      '';
                      # Grant CREATEDB so sqlx::test can create temp databases from the host.
                      services.postgresql.initialScript = pkgs.writeText "grant-createdb.sql" ''
                        ALTER USER horae CREATEDB;
                      '';

                      # Dev login: skip OIDC, auto-login as admin.
                      services.horae.secretKeyFile = null;
                      systemd.services.horae.environment.DEV_LOGIN = "1";

                      # Convenience login for debugging
                      services.openssh.enable = true;
                      services.openssh.settings.PermitRootLogin = "yes";
                      services.openssh.settings.PermitEmptyPasswords = "yes";
                      security.pam.services.sshd.allowNullPassword = true;
                      users.extraUsers.root.password = "";

                      # QEMU port forwards: host → guest
                      virtualisation.forwardPorts = [
                        { from = "host"; host.port = 2222; guest.port = 22; }
                        { from = "host"; host.port = 3000; guest.port = 3000; }
                        { from = "host"; host.port = 5432; guest.port = 5432; }
                      ];
                      virtualisation.memorySize = 1024;

                      # Open forwarded ports in the guest firewall
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
              meta.description = "Start the Horae dev VM (NixOS + Postgres via QEMU)";
            };
        });
    };
}
