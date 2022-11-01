# HD FPV font tool

This tool is for managing OSD fonts or tile collections for HD FPV systems (Walksnail Avatar, HDZero, DJI FPV system)

## How to use

3 formats are supported:
* bin files which are used on the goggles
* tiledir: directory containing tiles in individual PNG files with file name format %03d.png (000.png to 255.png)
* tilegrid: image file with tiles arranged in a 16x16 grid

It is possible to convert between all 3 formats with the command:
`hd_fpv_font_tool convert <from> <to>`

With from and to being in the format <format>:<path>

## Examples

### Extract tiles from a bin file to individual files for each tile

`hd_fpv_font_tool convert bin:font.bin tiledir:font_tiles`

Will extract all the tiles from `font.bin` to the `font_tiles` directory creating 256 files (000.png to 255.png)

### Extract tiles from a bin file to a tile grid image file (allows editing and also have an overview of the tiles)

`hd_fpv_font_tool convert bin:font.bin tilegrid:font_grid.png`

### Creating a bin file to use on your goggles from a tile grid or tile directory

* From a tile grid: `hd_fpv_font_tool convert tilegrid:font_grid.png bin:font.bin`
* From a tile directory: `hd_fpv_font_tool convert tiledir:font_tiles bin:font.bin`

## Future

* For now only the DJI FPV system fonts are supported but fonts for the Avatar and HDZero video systems will be supported soon
* Font symbols can span several tiles, soon it will be possible to extract the full symbols to a directory for easier editing
* Support for DJI FPV extended fonts (treating font_*.bin and font_*_2.bin) as a single collection of 512 tiles