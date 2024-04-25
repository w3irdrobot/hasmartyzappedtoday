{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { nixpkgs, fenix, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustPlatform = import fenix { inherit system; };
        rust = with rustPlatform; combine [
          complete.rustc
          complete.cargo
          complete.rust-src
          targets.wasm32-unknown-unknown.latest.rust-std
        ];
        craneLib = crane.lib.${system}.overrideToolchain rust;
      in
      {
        devShells.default = craneLib.devShell
          {
            packages = with pkgs; [
              dioxus-cli
              cargo-expand
              nodejs_20
            ];

            CC_wasm32_unknown_unknown = "${pkgs.llvmPackages_14.clang-unwrapped}/bin/clang-14";
            CFLAGS_wasm32_unknown_unknown = "-I ${pkgs.llvmPackages_14.libclang.lib}/lib/clang/14.0.6/include/";
            RUST_SRC_PATH = "${rustPlatform.complete.rust-src}/lib/rustlib/src/rust/library";
          };
      });
}
