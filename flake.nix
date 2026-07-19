{
  description = "typeless-ibus: native IBus voice input for Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "typeless-ibus";
            version = "0.4.0";
            src = pkgs.lib.cleanSourceWith {
              src = ./.;
              filter =
                path: type:
                pkgs.lib.cleanSourceFilter path type
                && !(builtins.elem (baseNameOf path) [
                  "result"
                  "target"
                ]);
            };

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [
              pkgs.alsa-lib
              pkgs.libopus
            ];

            postInstall = ''
              mkdir -p "$out/libexec" "$out/share/ibus/component" "$out/share/doc/typeless-ibus"
              mv "$out/bin/typeless-ibus-engine" "$out/libexec/typeless-ibus-engine"
              ln -s ../libexec/typeless-ibus-engine "$out/bin/typeless-ibus-engine"
              substitute data/typeless.xml "$out/share/ibus/component/typeless.xml" \
                --replace-fail /usr/libexec/typeless-ibus-engine "$out/libexec/typeless-ibus-engine"
              install -m644 data/config.example.json README.md README_zh.md CHANGELOG.md docs/THIRD_PARTY.md \
                -t "$out/share/doc/typeless-ibus"
            '';

            meta = {
              description = "Native IBus voice input engine for Linux";
              homepage = "https://github.com/day253/typeless-ibus";
              license = pkgs.lib.licenses.mit;
              mainProgram = "typeless-ibus-engine";
              platforms = systems;
            };
          };
        }
      );

      checks = forAllSystems (system: {
        package = self.packages.${system}.default;
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              pkg-config
              rustc
              rustfmt
            ];
            buildInputs = with pkgs; [
              alsa-lib
              libopus
            ];
          };
        }
      );

      formatter = forAllSystems (system: (import nixpkgs { inherit system; }).nixfmt);
    };
}
