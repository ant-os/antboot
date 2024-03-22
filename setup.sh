#!/usr/bin/env sh

rustup target add x86_64-unknown-uefi
rustup component add llvm-tools
mkdir .locale
cp /usr/share/OVMF/x64/OVMF_CODE.fd .
cp /usr/share/OVMF/x64/OVMF_VARS.fd .