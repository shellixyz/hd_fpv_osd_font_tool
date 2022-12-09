
test:
    cargo nextest run

build:
    cargo build --release

build-win:
    cargo build --release --target x86_64-pc-windows-gnu