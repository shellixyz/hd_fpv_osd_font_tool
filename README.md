# HD FPV font tool

This tool is for managing OSD fonts or tile collections for HD FPV systems (Walksnail Avatar, HDZero, DJI FPV system)

## How to use

Run `dji_fpv_font_tool --help` for commands help

## How to use examples

### Extract tiles from a DJI bin file to individual files for each tile

`hd_fpv_font_tool convert djibin:font.bin tiledir:font_tiles`

Will extract all the tiles from `font.bin` to the `font_tiles` directory creating 256 files (000.png to 255.png)

### Extract tiles from a DJI bin file to a tile grid image file (allows editing and also have an overview of the tiles)

`hd_fpv_font_tool convert djibin:font.bin tilegrid:font_grid.png`

### Creating a DJI bin file to use on your goggles from a tile grid or tile directory

* From a tile grid: `hd_fpv_font_tool convert tilegrid:font_grid.png djibin:font.bin`
* From a tile directory: `hd_fpv_font_tool convert tiledir:font_tiles djibin:font.bin`

## Building

* Install the Rust compiler/toolchain: [see here](https://www.rust-lang.org/tools/install)
* Clone the repository: `https://github.com/shellixyz/hd_fpv_font_tool.git`
* Build: `cd hd_fpv_font_tool && cargo build`

## Installing the latest version from source through Cargo

* Install the Rust compiler/toolchain: [see here](https://www.rust-lang.org/tools/install)
* Install: `cargo install --locked --git https://github.com/shellixyz/hd_fpv_osd_font_tool`

## Future

* For now only the DJI FPV system and Walksnail Avatar fonts are supported but fonts for the HDZero video systems will be supported soon

## Development

Install the Git hooks with `just install-hooks`. The `pre-commit` hook checks the staged tree:
ShellCheck on the hooks, `Cargo.lock`, man page generation, formatting, `cargo check`, Clippy,
`cargo doc` and the tests. On success, `commit-msg` adds a `Checked-Tree` trailer to the commit.

The hooks need [just](https://github.com/casey/just), ShellCheck, stable Rust with Clippy, and the
pinned formatting toolchain: `rustup toolchain install nightly-2026-02-12 --component rustfmt`.
