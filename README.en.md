# Vermouth

[日本語](README.md) | English

A system that hands out the password of a VeraCrypt-encrypted container from a server at home, so that the container is mounted automatically after the computer starts. It consists of three Rust programs. The rules of the behavior are in [docs/SPEC.md](docs/SPEC.md) (in Japanese).

## Server

The HTTP server that hands out the passwords (`vermouth-server`). The address to listen on is decided by, in this order: the arguments `<host> <port>`, the environment variable `LISTEN=host:port`, or `"listen"` in `config.json`. The config template is `server.example.config.json`.

The answer (the password) is encrypted and authenticated with keys made from the `token` (plain HTTP, no certificate needed; it is not TLS). Use a long random `token`, for example `openssl rand -base64 32`. Keep the client and the server on the same version (an old client cannot talk to a new server).

## Client

The client (`vermouth`) that fetches the password and runs VeraCrypt. It calls `client.linux.sh` on Linux and `client.windows.bat` on Windows. The config template is `client.example.env`.

## NanoKVM

Runs on a NanoKVM (a RISC-V KVM device), detects that the computer is powered on, and types the password as a USB keyboard (`vermouth-nanokvm`). The config template is `nanokvm.example.env`.

### Setup

The set of files to put on the NanoKVM is in `nanokvm/directory/` (the executable and `.env` in `/data/vermouth/`, `/etc/init.d/S99vermouth.sh`, `/etc/kvm/scripts/input-password.sh`). See [docs/SPEC.md](docs/SPEC.md) for the details.

## Download

The releases hold the executables for each platform (Linux x86_64 / aarch64 / riscv64, Windows x86_64, macOS aarch64).
