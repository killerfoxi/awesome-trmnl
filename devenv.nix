{ pkgs, lib, config, inputs, ... }:

{
  packages = [ pkgs.cargo-edit ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  tasks."atrmnl:build".exec = ''
    cargo build --release --bin atrmnl_server
    echo "Binary built at: target/release/atrmnl_server"
  '';
}
