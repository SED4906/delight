with import <nixpkgs> {}; {
  qpidEnv = stdenv.mkDerivation {
    name = "build-environment-delight";
    buildInputs = [
        rustup
        rust-analyzer
    ];
  };
}
