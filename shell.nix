{
  pkgs ? import <nixpkgs> {},
  lib,
  stdenv,
}: let
  linuxRuntime = with pkgs;
    lib.optionals (stdenv.hostPlatform.isLinux) [
      libxcursor
      libxrandr
      libxi
      libxkbcommon
      vulkan-loader
      wayland
      libGL
    ];
  runtimePaths =
    lib.makeLibraryPath linuxRuntime;
in
  pkgs.mkShell {
    buildInputs = with pkgs.buildPackages;
    with pkgs;
      [
        openssl
        pkg-config

        clang

        # Dynamic linking for crate deps
        rust-jemalloc-sys
        fontconfig
        freetype

        udev # Needed by libudev-sys for GamePad API.

        uv
        python3
        llvmPackages.bintools # provides lld

        (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)

        ## bevy deps
        alsa-lib.dev
        udev.dev
        # xorg
        libx11
        libxrandr
        libxcursor
        libxcb
        libxi

        wayland
        libxkbcommon
        libxkbcommon.dev
        vulkan-loader
        vulkan-tools
        glfw
        # notice this line might not match your needs or desires
        xorg.xf86videoamdgpu
      ]
      ++ linuxRuntime;

    shellHook = ''
      export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString runtimePaths}";
    '';

    # copy resources into `$out` to be used during runtime
    # link runtime libraries
    postFixup = ''
      mkdir -p $out/resources
      cp -r ./resources $out/

      wrapProgram $out/bin/servo \
        --prefix LD_LIBRARY_PATH : ${runtimePaths}
    '';

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
