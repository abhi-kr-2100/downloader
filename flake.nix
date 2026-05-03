{
  description = "Downloader - A simple way to download things via HTTP/HTTPS";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              cargo
              cargo-edit
              rustc
              rustfmt
              clippy
            ];

            buildInputs = with pkgs; [
              openssl
            ];
          };
        });
    };
}
