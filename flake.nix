{
  description = "shard - yt-dlp TUI downloader";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            # yt-dlp と ffmpeg を開発シェル内でも直接利用可能にする
            yt-dlp
            ffmpeg
            # arboard / クリップボード連携用ライブラリ
            libX11
            libXcursor
            libXrandr
            libXi
            wayland
            wl-clipboard
          ];
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "shard";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];

          buildInputs = with pkgs; [
            openssl
            libX11
            libXcursor
            libXrandr
            libXi
            wayland
          ];

          # yt-dlp と ffmpeg を実行時 PATH に自動バインド
          postInstall = ''
            wrapProgram $out/bin/shard \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.yt-dlp pkgs.ffmpeg ]}
          '';
        };
      }
    );
}
