#!/usr/bin/env sh

cargo build --target x86_64-unknown-uefi --release

mkdir -p esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/antboot-efi.efi esp/EFI/BOOT/BOOTX64.efi

qemu-system-x86_64 -enable-kvm \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -serial stdio
