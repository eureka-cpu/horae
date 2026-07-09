{ inputs, pkgs, ... }:
(inputs.treefmt.lib.evalModule pkgs ./treefmt.nix).config.build.wrapper
