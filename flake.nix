{
  description = "FOCAS-RS";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        pkgs32 = pkgs.pkgsi686Linux;

        libraries = with pkgs; [
          stdenv.cc.cc.lib
        ];

        packages = with pkgs; [
          rustToolchain

          # System Build Tools
          pkg-config
          llvmPackages.libclang
          clang
          cargo-xwin

        ] ++ libraries;
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [
            "i686-unknown-linux-gnu"
            "i686-pc-windows-msvc"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = packages;

          CC_i686_pc_windows_msvc = "${pkgs.clang}/bin/clang";
          CXX_i686_pc_windows_msvc = "${pkgs.clang}/bin/clang++";
          AR_i686_pc_windows_msvc = "${pkgs.llvmPackages.llvm}/bin/llvm-ar";
          RC_i686_pc_windows_msvc = "${pkgs.llvmPackages.llvm}/bin/llvm-rc";

          CRATE_CC_NO_DEFAULTS = "1";

          CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_LINKER = "${pkgs32.gcc}/bin/gcc";
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages.libclang.lib ];
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath libraries}:${./lib}:${pkgs32.lib.makeLibraryPath [ pkgs32.glibc pkgs32.stdenv.cc.cc.lib ]}";
          XDG_DATA_DIRS = "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS";
          CC_i686_unknown_linux_gnu = "${pkgs.pkgsi686Linux.stdenv.cc}/bin/cc";
          shellHook = ''
                        export PATH="${pkgs.llvmPackages.llvm}/bin:$PATH"
                        export XWIN_CACHE_DIR="$HOME/.cache/xwin"

                        export RUSTFLAGS="-L native=$XWIN_CACHE_DIR/xwin/crt/lib/x86 \
                              -L native=$XWIN_CACHE_DIR/xwin/sdk/lib/ucrt/x86 \
                              -L native=$XWIN_CACHE_DIR/xwin/sdk/lib/um/x86"
                        export CFLAGS_i686_pc_windows_msvc="-Wno-unused-command-line-argument"

                        export XWIN_CACHE_DIR="$HOME/.cache/xwin"
                        export RUSTFLAGS="-L native=$XWIN_CACHE_DIR/xwin/crt/lib/x86 \
                        -L native=$XWIN_CACHE_DIR/xwin/sdk/lib/ucrt/x86 \
                        -L native=$XWIN_CACHE_DIR/xwin/sdk/lib/um/x86"

                        export BINDGEN_EXTRA_CLANG_ARGS="--target=i686-pc-windows-msvc"
          '';
        };
      }
    );
}
