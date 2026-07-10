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

      # Overlay that rebuilds horae against the consumer's nixpkgs (composing
      # fenix), so cross variants resolve. Used both as overlays.shared-nixpkgs
      # and to build the legacyPackages instances below.
      sharedNixpkgsOverlay = import ./nix/overlays/shared-nixpkgs.nix {
        inherit lib;
        inherit (blueprint) mkPackagesFor;
        fenix = inputs.fenix.overlays.default;
      };
    in
    blueprint
    // {
      # Expose horae as a nixpkgs overlay for downstream flakes:
      #   default        — reuse blueprint's prebuilt packages (cache-friendly)
      #   shared-nixpkgs — rebuild against the consumer's nixpkgs, so cross
      #                    compilation works (e.g. pkgsCross.<target>.horae)
      overlays = {
        default = import ./nix/overlays/default.nix {
          inherit (blueprint) packages;
        };
        shared-nixpkgs = sharedNixpkgsOverlay;
      };

      # nixpkgs instances with the shared-nixpkgs overlay applied, so horae and
      # its cross variants are reachable directly, e.g.
      # `nix build .#legacyPackages.aarch64-darwin.pkgsCross.aarch64-multiplatform.horae`.
      legacyPackages = lib.genAttrs systems (system:
        import inputs.nixpkgs {
          inherit system;
          overlays = [ sharedNixpkgsOverlay ];
        });

      apps = lib.genAttrs systems (system:
        (blueprint.apps.${system} or { }) // {
          preview-site =
            let
              pkgs = inputs.nixpkgs.legacyPackages.${system};
              script = pkgs.writeShellApplication {
                name = "preview-site";
                runtimeInputs = with pkgs; [ python3 git ];
                text = ''
                  port=''${1:-8080}
                  root=$(git rev-parse --show-toplevel)
                  echo "Serving site at http://localhost:$port"
                  exec python3 -m http.server "$port" --directory "$root/site"
                '';
              };
            in
            {
              type = "app";
              program = "${script}/bin/preview-site";
              meta.description = "Serves the static site locally.";
            };

          postgres =
            let
              hostPkgs = inputs.nixpkgs.legacyPackages.${system};
              guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
              debugConfig = lib.nixosSystem {
                system = null;
                modules = [
                  "${inputs.nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
                  ({ config, lib, ... }:
                    {
                      services.postgresql = {
                        enable = true;
                        enableTCPIP = true;
                        ensureDatabases = [ "horae" ];
                        ensureUsers = [
                          {
                            name = "horae";
                            ensureDBOwnership = true;
                            ensureClauses = {
                              createdb = true;
                              login = true;
                            };
                          }
                        ];
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
                        host.pkgs = hostPkgs;
                        forwardPorts = with config.services;
                          lib.optional openssh.enable
                            {
                              from = "host";
                              host.port = 2222;
                              guest.port = 22;
                            }
                          ++ lib.optional postgresql.enable
                            {
                              from = "host";
                              host.port = postgresql.settings.port;
                              guest.port = postgresql.settings.port;
                            }
                        ;
                      };
                      networking.firewall.allowedTCPPorts =
                        with config.services; lib.optional postgresql.enable postgresql.settings.port;

                      environment.systemPackages =
                        with config.services; lib.optional postgresql.enable postgresql.package;

                      nixpkgs.hostPlatform = guestSystem;
                      system.stateVersion = lib.trivial.release;
                    })
                ];
              };
            in
            {
              type = "app";
              program = "${debugConfig.config.system.build.vm}/bin/run-nixos-vm";
              meta.description = ''
                Launch a NixOS VM preconfigured with PostgreSQL and SSH for Horae development.
                The VM state is persisted in the generated `nixos.qcow2` file; you can wipe the
                VM state completely by removing this file. This instance serves as the required
                database container for running `sqlx` tests and provides root access with port
                forwarding enabled. Note: Migrations are not automatic; please refer to the
                Getting Started guide for instructions on seeding data and running migrations.
              '';
            };
        });
    };
}
