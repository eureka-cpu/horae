{ inputs, pkgs, ... }:
(inputs.treefmt.lib.evalModule pkgs ../treefmt.nix).config.build.check inputs.self
