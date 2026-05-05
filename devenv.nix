{ pkgs, lib, config, inputs, ... }:

{
  packages = [
    pkgs.cargo-edit
    pkgs.pkgsStatic.stdenv.cc
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
    targets = [ "x86_64-unknown-linux-musl" ];
  };

  tasks."atrmnl:build".exec = ''
    cargo build --release --bin atrmnl_server
    echo "Binary built at: target/release/atrmnl_server"
  '';

  tasks."atrmnl:build:alpine".exec = ''
    cargo build --release --target x86_64-unknown-linux-musl --bin atrmnl_server
    echo "Alpine/musl binary built at: target/x86_64-unknown-linux-musl/release/atrmnl_server"
  '';

}
