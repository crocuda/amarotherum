{
  pkgs ? import <nixpkgs> {},
  lib,
  ...
}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "amarotherum";
  version = (builtins.fromTOML (lib.readFile ./${pname}/Cargo.toml)).package.version;

  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
    # outputHashes = {
    #   "tappers-0.4.2" = "sha256-kx/gLngL7+fH5JmJTVTGawyNdRde59dbFdrzermy/CE=";
    # };
  };

  # disable tests
  checkType = "debug";
  doCheck = false;

  nativeBuildInputs = with pkgs; [
    installShellFiles
    pkg-config

    clang

    # Dynamic linking for crate deps
    rust-jemalloc-sys
    fontconfig
    freetype
  ];
  buildInputs = with pkgs; [
    openssl
    pkg-config

    uv
    python3
    llvmPackages.bintools # provides lld

    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
  ];

  MACH_USE_NIX = true;
  UV_PYTHON = pkgs.python3;
  UV_VENV_CLEAR = 1; # clear env no confirm

  LIBCLANG_PATH = with pkgs;
    lib.makeLibraryPath [
      # libclang
      llvmPackages.clang-unwrapped.lib
      # pkgs.llvmPackages.libclang.lib
    ];
  CLANG_PATH = with pkgs; lib.makeLibraryPath [llvmPackages.clang-unwrapped.lib];
  # Enable colored cargo and rustc output
  TERMINFO = with pkgs; "${ncurses.out}/share/terminfo";
}
