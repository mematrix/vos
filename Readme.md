# vOS
A RISC-V Operator System written by Rust.

# Minimum Supported Rust Version (MSRV)
Some features this project used are still unstable, so a nightly version is required.

- Min Version: `1.65.0`.
  * feature `const_ptr_offset_from` is stable since `1.65.0`.
- Nightly Version:
  * feature `default_alloc_error_handler` is unstable. Needed with the custom `GlobalAllocator` and `alloc` crate.
  * feature `inline_const` is unstable. Needed on the custom impl of `offset_of!` macro.

# Build
1. Make sure the rust version meets the minimum requirements.
2. Make sure the RISC-V target has been installed.
   > ```shell
   > # rustup default nightly
   > rustup target add riscv64gc-unknown-none-elf
   > ```
3. Run `cargo build` to build the project.

# Run
Before the run, make sure the file named `hdd.dsk` is existed in the project root dir, this file is used in `QEMU` as the disk device (See `QEMU` arguments).

A way to create the `hdd.dsk` file:

```shell
dd if=/dev/zero of=hdd.dsk count=32 bs=1M
```

The simplest way to launch the kernel is using `cargo run` command, this will use the **cargo config** in `.cargo/config.toml` to run with `QEMU`.

Another way is to launch `QEMU` manually, for example:

```shell
qemu-system-riscv64 -machine virt -cpu rv64 -smp 4 -m 128M -nographic -s -drive if=none,format=raw,file=hdd.dsk,id=foo -device virtio-blk-device,drive=foo -serial mon:stdio -bios none -kernel ./target/riscv64gc-unknown-none-elf/debug/vos
```

The `QEMU` arguments can be customized.

# License
This project is under the `MIT` license ([LICENSE](./LICENSE)) and any dependence is using its original license.
