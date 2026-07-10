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
  shellHook = ''
    # Use the port forwarded and postgres user created by the VM
    export DATABASE_URL=postgres://horae@127.0.0.1:5432/horae
  '';
}
