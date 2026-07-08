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
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildFeatures = [ "server" ];
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
              pkgs.wasm-pack
              pkgs.nil
            ];
            RUST_SRC_PATH = "${fenixPkgs.stable.rust-src}/lib/rustlib/src/rust/library";
          };
        }
      );

      formatter = forAllSystems (system: treefmtEval.${system}.config.build.wrapper);

      checks = forAllSystems (system: {
        fmt = treefmtEval.${system}.config.build.check self;
      });

      nixosModules.default = {
        imports = [ ./nixos/modules/horae/default.nix ];
        _module.args.self = self;
      };

      nixosConfigurations.default = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit self; };
        modules = [
          self.nixosModules.default
          (
            { ... }:
            {
              services.horae.enable = true;
              services.horae.database.createLocally = true;

              # Minimal bootable VM config
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
}
