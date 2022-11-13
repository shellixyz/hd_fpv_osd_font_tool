
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::exit;

use clap::{Parser, Subcommand};

use hd_fpv_osd_font_tool::prelude::*;
use hd_fpv_osd_font_tool::log_level::LogLevel;

mod convert;
mod convert_set;

use convert::convert_command;
use convert_set::convert_set_command;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {

    #[clap(short, long, value_parser, default_value_t = LogLevel::Info)]
    #[arg(value_enum)]
    log_level: LogLevel,

    #[command(subcommand)]
    command: Commands,

}

#[derive(Subcommand)]
enum Commands {
    /// Converts between tile collection formats
    ///
    /// Valid collection specifications are:{n}
    ///     * bin:path          raw RGBA file{n}
    ///     * tilegrid:path     grid of tiles image{n}
    ///     * tiledir:path      directory with each tile in a separate file{n}
    ///     * symdir:path       directory with each symbol in a separate file{n}
    ///
    /// Bin files normalized names{n}
    ///     Generic bin files (no ident):{n}
    ///         SD: font.bin + font2.bin{n}
    ///         HD: font_hd.bin + font_hd_2.bin{n}
    ///     With ident:{n}
    ///         SD: font_<ident>.bin + font_<ident>_2.bin{n}
    ///         HD: font_<ident>_hd.bin + font_<ident>_hd_2.bin{n}
    ///
    /// Tile directory (tiledir){n}
    ///     A tile directory is a directory representing a collection of tiles with each tile in a separate file. Each file{n}
    ///     is named from the index of the tile 0 padded to 3 digits and with the png extensions e.g. 011.png
    ///
    /// Symbol directory (symdir){n}
    ///     A symbol is a small sub-collection of tiles representing a full symbol (symbol spanning across several tiles).{n}
    ///     When saving to a symdir the symbol specifications file can be specified with the -s/--symbols-specs-file argument.{n}
    ///     A symbol directory contains every symbol of the collection with specific name formats:{n}
    ///     - symbols spanning a single tile: index of the symbol 0 padded to 3 digits and with png extension e.g. 011.png{n}
    ///     - other symbols: index of the first tile and index of the last tile 0 padded to 3 digits and separated by `-` e.g. 030-032.png
    ///
    /// Example: extracting the tiles from a bin file to individual files in the `tiles` directory:{n}
    ///     `convert bin:font.bin tiledir:tiles`
    Convert {

        #[clap(short, long, value_parser, default_value = "sym_specs.yaml")]
        symbol_specs_file: PathBuf,

        /// source collection in the form of a tile collection specification, see above
        from: String,

        /// destination collection in the form of a tile collection specification, see above
        to: String
    },

    /// Converts between tile collection set formats
    ///
    /// A collection set contains both SD and HD tiles/symbols
    ///
    /// Valid collection specifications are:{n}
    ///     * binset:sd_path:sd_2_path:hd_path:hd_2_path{n}
    ///     * binsetnorm:path:ident         set of bin files with normalized names{n}
    ///     * tilesetgrids:sd_path:hd_path  grids of tiles image forming a SD/HD set{n}
    ///     * tilesetgridsnorm:path:ident   grid of tiles image set with normalized names{n}
    ///     * tilesetdir:path               directory with SD and HD tiles in the corresponding directory{n}
    ///     * symsetdir:path                directory with SD and HD symbols in the corresponding directory
    ///
    /// Bin files normalized names (binsetnorm){n}
    ///     Generic bin files (no ident):{n}
    ///         SD: font.bin + font2.bin{n}
    ///         HD: font_hd.bin + font_hd_2.bin{n}
    ///     With ident:{n}
    ///         SD: font_<ident>.bin + font_<ident>_2.bin{n}
    ///         HD: font_<ident>_hd.bin + font_<ident>_hd_2.bin{n}
    ///     If `path/indent` is not provided will read the files from the current directory without ident
    ///
    /// Grid files normalized names{n}
    ///     Generic grid image files (no ident):{n}
    ///         SD: grid.png{n}
    ///         HD: grid_hd.bin{n}
    ///     With ident:{n}
    ///         SD: grid_<ident>.png{n}
    ///         HD: grid_<ident>_hd.png
    ///
    /// Tile/symbol sets directory (tilesetdir / symsetdir){n}
    ///     A directory with the SD tiles in the SD subdirectory and HD tiles in the HD subdirectory{n}
    ///     When saving to a symsetdir the symbol specifications file can be specified with the -s/--symbols-specs-file argument.{n}
    ///     If `path/indent` is not provided will read the files from the current directory without ident
    ///
    /// Example: extracting the tiles from a bin file set with normalized name and no ident from the `font_files` directory{n}
    ///          to individual files. SD tiles in the `tiles/SD` directory and HD tiles in the `tiles/HD` directory:{n}
    ///     `convert-set binsetnorm:font_files tiledir:tiles`
    ConvertSet {

        #[clap(short, long, value_parser, default_value = "sym_specs.yaml")]
        symbol_specs_file: PathBuf,

        /// source collection in the form of a tile collection specification, see above
        from: String,

        /// destination collection in the form of a tile collection specification, see above
        to: String
    },
}

pub struct ConvertOptions<'a> {
    symbol_specs_file: &'a PathBuf
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {
        Commands::Convert { from, to, symbol_specs_file } => convert_command(from, to, ConvertOptions { symbol_specs_file }),
        Commands::ConvertSet { from, to, symbol_specs_file } => convert_set_command(from, to, ConvertOptions { symbol_specs_file }),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
