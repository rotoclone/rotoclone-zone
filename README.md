# rotoclone-zone

The source code for my personal website

## Building for Raspberry Pi from Windows
1. Get linker from https://gnutoolchains.com/raspberry/
1. Add target: `rustup target add armv7-unknown-linux-gnueabihf`
1. Build: `cargo build --release --target=armv7-unknown-linux-gnueabihf`