{ lib, fetchFromGitHub, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "tui-tree-select";
  version = "0.0.1";

  # cargoLock = {
  #   lockFile = ./Cargo.lock;
  # };


  cargoSha256 = lib.fakeSha256;
  # cargoHash = "";

  src = ./.;
}
