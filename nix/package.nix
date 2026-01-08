{
  lib,
  naersk,
  stdenv,
  pkg-config,
  openssl,
}:

naersk.buildPackage {
  pname = "dwrs";
  version = "0.2.1";
  src = lib.cleanSource ../.;
  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  # cargoBuildOptions =
  #   opts:
  #   opts
  #   ++ [
  #     "--bin"
  #     "dwrs"
  #   ];

  meta = with lib; {
    description = "Parallel file downloader with progress bar and i18n";
    homepage = "https://github.com/Bircoder432/dwrs";
    license = licenses.asl20;
    platforms = platforms.linux;
    mainProgram = "dwrs";
  };
}
