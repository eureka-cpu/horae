_: {
  projectRootFile = "flake.nix";

  programs = {
    # Rust
    rustfmt.enable = true;

    # Nix: lint (deadnix, statix) then format (nixpkgs-fmt). Ordered below.
    deadnix.enable = true;
    statix.enable = true;
    nixpkgs-fmt.enable = true;

    # TOML
    taplo.enable = true;

    # Markdown
    mdformat.enable = true;

    # JSON
    jsonfmt.enable = true;

    # YAML formatting + GitHub Actions workflow linting. Keep single blank
    # lines (e.g. between workflow steps) rather than collapsing them.
    yamlfmt.enable = true;
    yamlfmt.settings.formatter.retain_line_breaks_single = true;
    actionlint.enable = true;

    # Shell: lint (shellcheck) then format (shfmt). Ordered below.
    shellcheck.enable = true;
    shfmt.enable = true;
  };

  settings = {
    # Leave generated / vendored trees alone: skill and spec-kit content is
    # managed by their tools, so formatting or linting them only causes churn
    # (and shellcheck would fail on the vendored scripts).
    global.excludes = [
      "skills-lock.json"
      ".specify/**"
      ".agents/**"
      ".claude/skills/**"
      ".sqlx/**"
    ];

    # When several tools touch the same files, group them into a pipeline and
    # order by priority (lower runs first): lint/rewrite first, format last.
    formatter = {
      deadnix.pipeline = "nix";
      deadnix.priority = 1;
      statix.pipeline = "nix";
      statix.priority = 2;
      nixpkgs-fmt.pipeline = "nix";
      nixpkgs-fmt.priority = 3;

      shellcheck.pipeline = "shell";
      shellcheck.priority = 1;
      shfmt.pipeline = "shell";
      shfmt.priority = 2;
    };
  };
}
