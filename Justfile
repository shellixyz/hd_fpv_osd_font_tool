install-hooks:
    git config core.hooksPath .githooks
    @echo "Installed hd_fpv_osd_font_tool Git hooks."

test:
    cargo nextest run

build:
    cargo build --release

build-win:
    cargo build --release --target x86_64-pc-windows-gnu
