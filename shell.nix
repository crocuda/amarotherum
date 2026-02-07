{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs.buildPackages; [
    openssl
    pkg-config
    iproute2

    # rust vmm uses latest stable and oxalica tend to lag behind.break
    # so we temporary force use of beta.

    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
    rust-analyzer
  ];
  # SeaOrm Sqlite database
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
}
