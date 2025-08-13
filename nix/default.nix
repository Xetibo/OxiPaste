{
  cargo,
  cargo-watch,
  clippy,
  dbus,
  glib,
  lib,
  libGL,
  libX11,
  libXcursor,
  libXi,
  libXrandr,
  libclang,
  libxkbcommon,
  lockFile,
  pkg-config,
  rust-analyzer,
  rustPlatform,
  rustc,
  vulkan-loader,
  wayland,
  wayland-protocols,
  ...
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  libPath = lib.makeLibraryPath [
    libGL
    libxkbcommon
    wayland
    pkg-config
    libclang
  ];
in
  rustPlatform.buildRustPackage rec {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;

    src = ../.;

    buildInputs = [
      dbus
      glib
      libGL
      libclang
      libxkbcommon
      pkg-config
      wayland
    ];

    cargoLock = {
      inherit lockFile;
      outputHashes = {
        "cryoglyph-0.1.0" = "sha256-Jc+rhzd5BIT7aYBtIfsBFFKkGChdEYhDHdYGiv4KE+c=";
        "dpi-0.1.1" = "sha256-hlVhlQ8MmIbNFNr6BM4edKdZbe+ixnPpKm819zauFLQ=";
        "iced-0.14.0-dev" = "sha256-ToInrksjWeUj7yKF4I7/GOD883abHX6WrmADCZrOa80=";
        "iced_exdevtools-0.14.0-dev" = "sha256-1ncfSYSeHUl59cGchpbXyAh/IB6Mxse6D3P5CLRh9kE=";
        "oxiced-0.5.1" = "sha256-U8gYs3Xzvso0QdDapOTgR3sPPMDjdPc7jwbI32o3TyE=";
      };
    };

    nativeBuildInputs = [
      cargo
      cargo-watch
      clippy
      glib
      libGL
      libclang
      libxkbcommon
      pkg-config
      rust-analyzer
      rustc
      wayland
    ];

    copyLibs = true;
    LD_LIBRARY_PATH = libPath;
    LIBCLANG_PATH = "${libclang.lib}/lib";

    postInstall = ''
      install -D --mode=444 $src/${pname}.desktop $out/share/applications/${pname}.desktop
      install -D --mode=444 $src/assets/* -t $out/share/pixmaps/${pname}
    '';

    postFixup = let
      libPath = lib.makeLibraryPath [
        libGL
        libX11
        libXcursor
        libXi
        libXrandr
        libxkbcommon
        vulkan-loader
        wayland
        wayland-protocols
      ];
    in ''
      patchelf --set-rpath "${libPath}" "$out/bin/oxipaste"
      patchelf --set-rpath "${libPath}" "$out/bin/oxipaste_daemon"
      patchelf --set-rpath "${libPath}" "$out/bin/oxipaste_command_runner"
    '';

    meta = with lib; {
      description = "Simple clipboard manager written in Rust/Iced";
      homepage = "https://github.com/DashieTM/OxiPaste";
      changelog = "https://github.com/DashieTM/OxiPaste/releases/tag/${version}";
      license = licenses.gpl3;
      maintainers = with maintainers; [DashieTM];
      mainProgram = "oxipaste";
    };
  }
