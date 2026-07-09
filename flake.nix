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
    {
      self,
      nixpkgs,
      treefmt-nix,
      fenix,
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
          # When building the Linux package from a Darwin host, cross-compile
          # to aarch64-unknown-linux-gnu using the Darwin toolchain + Linux sysroot.
          # On a native Linux host (or via a Linux builder), this is a no-op.
          buildPkgs =
            if system == "aarch64-linux" && pkgs.stdenv.hostPlatform.isDarwin
            then pkgs.pkgsCross.aarch64-multiplatform
            else pkgs;
          toolchain = fenix.packages.${system}.stable.withComponents [
            "rustc"
            "cargo"
            "rust-std"
          ];
          rustPlatform = buildPkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "horae";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildFeatures = [ "server" ];
            # Integration tests require a live Postgres (sqlx::test creates temp DBs).
            # They run via `cargo test --features server` with DATABASE_URL set locally
            # or in the nixosTest e2e check — skip them in the package build.
            checkFlags = [ "--skip" "integration" ];
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
          pkgs = nixpkgs.legacyPackages.${system};
          # nixosTest builds a Linux VM — on Darwin this is handled by the
          # nix-darwin Linux builder, so the test is available on all systems.
          guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
          testPkgs = nixpkgs.legacyPackages.${guestSystem};
        in
        {
          fmt = treefmtEval.${system}.config.build.check self;

          # NixOS e2e test: golden path from boot to Harvest API query.
          # Run with: nix build .#checks.<system>.e2e
          e2e = testPkgs.testers.nixosTest {
            name = "horae-e2e";
            nodes.server = { ... }: {
              imports = [ self.nixosModules.default ];
              services.horae.enable = true;
              services.horae.database.createLocally = true;
              systemd.services.horae.environment.DEV_LOGIN = "1";
            };
            testScript = ''
              server.start()
              server.wait_for_unit("horae.service")
              server.wait_for_open_port(3000)

              # Health check
              server.succeed("curl -sf http://localhost:3000/health | grep -q ok")

              # Seed data
              server.succeed("horae seed")

              # Dev login: POST /auth/dev-login → should redirect (303) and set a cookie
              server.succeed(
                "curl -sf -o /dev/null -w '%{http_code}' -X POST http://localhost:3000/auth/dev-login | grep -q 303"
              )

              # Full login flow with cookie jar
              server.succeed(
                "curl -sf -c /tmp/cookies.txt -L -X POST http://localhost:3000/auth/dev-login -o /dev/null"
              )

              # Harvest API: list time entries (session-authenticated)
              result = server.succeed(
                "curl -sf -b /tmp/cookies.txt http://localhost:3000/harvest/v2/time_entries"
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
      nixosConfigurations = (forAllSystems (system:
        let
          # Dev VM: NixOS with Postgres, port-forwarded to localhost.
          # `virtualisation.host.pkgs` makes the run script and QEMU come from
          # the host (e.g. aarch64-darwin) while the guest boots aarch64-linux.
          hostPkgs = nixpkgs.legacyPackages.${system};
          guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
        in
        {
          dev = nixpkgs.lib.nixosSystem {
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

                  # Minimal bootable VM config
                  boot.loader.grub.device = "nodev";
                  fileSystems."/" = {
                    device = "none";
                    fsType = "tmpfs";
                    options = [ "mode=0755" ];
                  };

                  environment.systemPackages = [ pkgs.postgresql ];

                  system.stateVersion = "25.05";
                }
              )
            ];
          };
        }
      )) // {
        default = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = { inherit self; };
          modules = [
            self.nixosModules.default
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
        let
          vm = self.nixosConfigurations.${system}.dev.config.system.build.vm;
        in
        {
          dev-vm = {
            type = "app";
            program = "${vm}/bin/run-nixos-vm";
          };
        });
    };
}
