{ packages
,
}:
# Reuse blueprint's prebuilt packages (built against blueprint's own nixpkgs),
# so consumers get binary-cache hits. Keyed by the consumer's system.
final: _prev: {
  horae = packages.${final.stdenv.hostPlatform.system}.default;
}
