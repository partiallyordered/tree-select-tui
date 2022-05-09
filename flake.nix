# Cobbled together from
# - https://github.com/NixOS/nixpkgs/blob/f0255a6fce5343faa1cb878c74bc3644dd2379c0/doc/languages-frameworks/rust.section.md
# - https://github.com/srid/rust-nix-template/blob/bc84c6744da667254ada1d93fe3693d972c8f683/flake.nix
{
  description = "A TUI tree selection mechanism";

  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/nixos-unstable;
  };

  outputs = { self, nixpkgs }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        # overlays = [
        #   rust-overlay.overlay
        #   (self: super: {
        #     # Because rust-overlay bundles multiple rust packages into one
        #     # derivation, specify that mega-bundle here, so that crate2nix
        #     # will use them automatically.
        #     rustc = self.rust-bin.${rustChannel}.latest.default;
        #     cargo = self.rust-bin.${rustChannel}.latest.default;
        #   })
        # ];
      };
    in
    {
      packages.x86_64-linux.tui-tree-select = pkgs.callPackage ./default.nix {};
      defaultPackage.x86_64-linux = self.packages.x86_64-linux.tui-tree-select;
    };
}
