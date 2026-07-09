{ pkgs, perSystem, ... }:
pkgs.mkShell {
  # Pull in the toolchain and build inputs from the horae package.
  inputsFrom = [ perSystem.self.default ];
  packages = with pkgs; [
    dioxus-cli
    sqlx-cli
    postgresql
    wasm-pack # NOTE: wasm-bindgen version must match exactly
    nil
  ];
}
