# Keyberon-based keyboard firmware for the Dario's keyboards

This is a keyboard firmware for Dario's keyboards based on [keyberon](https://github.com/TeXitoi/keyberon).

## Compiling

Install the rust toolchain

```shell
curl https://sh.rustup.rs -sSf | sh
rustup target add thumbv7em-none-eabihf
rustup component add llvm-tools-preview
cargo install cargo-binutils
cargo install flip-link
```
Moreover, you need the `dfu-util` binary which can normally be installed with your linux distribution's package manager.

Compile the firmware for the split keyboard with
```shell
cargo objcopy --release --bin split -- -O binary keyberon.bin
```
or for the unibody keyboard
```shell
cargo objcopy --release --bin unibody -- -O binary keyberon.bin
```


## Flashing using DFU

Put the developement board in DFU mode by holding reset and then pushing
and releasing boot. Then flash it:
```shell
dfu-util -a 0 --dfuse-address 0x08000000 -D keyberon.bin
```

## Development
Ensure that the debugging probe (e.g. STLink V2) has user access rights (see https://embedded-trainings.ferrous-systems.com/installation.html#linux-only-usb)

```shell
cargo install probe-run
cargo install probe-rs-cli
```

Running the program with log output:
```shell
DEFMT_LOG=info cargo run
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

