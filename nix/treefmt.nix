_: {
  projectRootFile = "flake.nix";
  programs = {
    nixpkgs-fmt.enable = true;
    rustfmt.enable = true;
    taplo.enable = true;
    mdformat.enable = true;
  };
}
