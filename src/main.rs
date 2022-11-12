
#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::cmp::Ordering;

use clap::{Parser, Subcommand};

use derive_more::{Display, From, Error};
use hd_fpv_osd_font_tool::prelude::*;
use hd_fpv_osd_font_tool::log_level::LogLevel;

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

#[derive(Debug)]
enum InvalidConvertArgError {
    InvalidPrefix(String),
    InvalidImageFileExtension {
        path: String,
        extension: Option<String>
    },
    InvalidPath(String),
    NoPrefix
}

impl Error for InvalidConvertArgError {}

impl Display for InvalidConvertArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use InvalidConvertArgError::*;
        match self {
            InvalidPrefix(prefix) => write!(f, "invalid prefix: {}", prefix),
            NoPrefix => f.write_str("no prefix"),
            InvalidImageFileExtension { path, extension: Some(extension) } => write!(f, "invalid image file extension `{}`: {}", extension, path),
            InvalidImageFileExtension { path, extension: None } => write!(f, "image path has no file extension: {}", path),
            InvalidPath(path) => write!(f, "invalid path: {}", path),
        }
    }
}

enum ConvertArg<'a> {
    BinFile(&'a str),
    TileGrid(&'a str),
    TileDir(&'a str),
    SymbolDir(&'a str),
}

impl<'a> ConvertArg<'a> {

    fn prefix(&self) -> &'static str {
        use ConvertArg::*;
        match self {
            BinFile(_) => "bin",
            TileGrid(_) => "tilegrid",
            TileDir(_) => "tiledir",
            SymbolDir(_) => "symdir",
        }
    }
}

fn check_arg_image_file_extension(path: &str) -> Result<(), InvalidConvertArgError> {
    match Path::extension(Path::new(path)) {
        Some(os_str) => match os_str.to_str() {
            Some("png") => Ok(()),
            Some(extension) => Err(InvalidConvertArgError::InvalidImageFileExtension { path: path.to_owned(), extension: Some(extension.to_owned()) }),
            None => Err(InvalidConvertArgError::InvalidPath(path.to_owned()))
        },
        None => Err(InvalidConvertArgError::InvalidImageFileExtension { path: path.to_owned(), extension: None })
    }
}

fn identify_convert_arg(input: &str) -> Result<ConvertArg, InvalidConvertArgError> {
    if let Some(path) = input.strip_prefix("bin:") {
        Ok(ConvertArg::BinFile(path))
    } else if let Some(path) = input.strip_prefix("tilegrid:") {
        Ok(ConvertArg::TileGrid(path))
    } else if let Some(path) = input.strip_prefix("tiledir:") {
        Ok(ConvertArg::TileDir(path))
    } else if let Some(path) = input.strip_prefix("symdir:") {
        Ok(ConvertArg::SymbolDir(path))
    } else if let Some((prefix, _)) = input.split_once(':') {
        Err(InvalidConvertArgError::InvalidPrefix(prefix.to_owned()))
    } else {
        Err(InvalidConvertArgError::NoPrefix)
    }
}

#[derive(Debug)]
enum ConvertError {
    FromArg(InvalidConvertArgError),
    ToArg(InvalidConvertArgError),
    InvalidConversion {
        from_prefix: String,
        to_prefix: String
    }
}

impl Error for ConvertError {}

impl Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConvertError::*;
        match self {
            FromArg(error) => write!(f, "invalid `from` argument: {}", error),
            ToArg(error) => write!(f, "invalid `to` argument: {}", error),
            InvalidConversion { from_prefix, to_prefix } => write!(f, "invalid conversion from {} to {}", from_prefix, to_prefix),
        }
    }
}

struct ConvertOptions<'a> {
    symbol_specs_file: &'a PathBuf
}

fn convert_command(from: &str, to: &str, options: ConvertOptions) -> Result<(), ConvertError> {
    let from_arg = identify_convert_arg(from).map_err(ConvertError::FromArg)?;
    let to_arg = identify_convert_arg(to).map_err(ConvertError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertArg::*;
    match (&from_arg, &to_arg) {
        (BinFile(_), BinFile(_)) | (TileGrid(_), TileGrid(_)) | (TileDir(_), TileDir(_)) | (SymbolDir(_), SymbolDir(_)) =>
            return Err(ConvertError::InvalidConversion { from_prefix: from_arg.prefix().to_owned(), to_prefix: to_arg.prefix().to_owned()}),

        (BinFile(from_path), to_arg) => {
            let bin_file_tiles = bin_file::load(from_path).unwrap();
            match to_arg {
                TileGrid(to_path) => {
                    check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
                    bin_file_tiles.save_to_grid_image(to_path).unwrap()
                },
                TileDir(to_path) => bin_file_tiles.save_tiles_to_dir(to_path).unwrap(),
                SymbolDir(to_path) => {
                    let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
                    bin_file_tiles.to_symbols(&sym_specs).unwrap().save_to_dir(to_path).unwrap();
                },
                _ => unreachable!()
            }
        },

        (TileGrid(from_path), to_arg) => {
            check_arg_image_file_extension(from_path).map_err(ConvertError::FromArg)?;
            let tile_grid = crate::TileGrid::load_from_image(from_path).unwrap();
            match to_arg {
                BinFile(to_path) => tile_grid.save_tiles_to_bin_file(to_path).unwrap(),
                TileDir(to_path) => tile_grid.save_tiles_to_dir(to_path).unwrap(),
                SymbolDir(to_path) => {
                    let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
                    tile_grid.to_symbols(&sym_specs).unwrap().save_to_dir(to_path).unwrap();
                },
                _ => unreachable!()
            }
        },

        (TileDir(from_path), to_arg) => {
            let tiles = load_tiles_from_dir(from_path, 512).unwrap();
            match to_arg {
                BinFile(to_path) => tiles.save_to_bin_file(to_path).unwrap(),
                TileGrid(to_path) => {
                    check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
                    tiles.save_to_grid_image(to_path).unwrap()
                },
                SymbolDir(to_path) => {
                    let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
                    tiles.to_symbols(&sym_specs).unwrap().save_to_dir(to_path).unwrap();
                },
                _ => unreachable!()
            }
        },

        (SymbolDir(from_path), to_arg) => {
            let tiles = load_symbols_from_dir(from_path, 512).unwrap().into_tiles_vec();
            match to_arg {
                BinFile(to_path) => tiles.save_to_bin_file(to_path).unwrap(),
                TileGrid(to_path) => tiles.into_tile_grid().generate_image().unwrap().save(to_path).unwrap(),
                TileDir(to_path) => tiles.save_tiles_to_dir(to_path).unwrap(),
                _ => unreachable!()
            }
        }

    }

    Ok(())
}

enum ConvertSetArg<'a> {
    BinFileSet {
        sd_path: &'a str,
        sd_2_path: &'a str,
        hd_path: &'a str,
        hd_2_path: &'a str,
    },
    BinFileSetNorm {
        dir: &'a str,
        ident: Option<&'a str>
    },
    TileSetGrids {
        sd_path: &'a str,
        hd_path: &'a str,
    },
    TileSetGridsNorm {
        dir: &'a str,
        ident: Option<&'a str>
    },
    TileSetDir(&'a str),
    SymbolSetDir(&'a str),
}

impl<'a> ConvertSetArg<'a> {

    fn prefix(&self) -> &'static str {
        use ConvertSetArg::*;
        match self {
            BinFileSet {..} => "binset",
            BinFileSetNorm {..} => "binsetnorm",
            TileSetGrids {..} => "tilesetgrids",
            TileSetGridsNorm {..} => "tilesetgridsnorm",
            TileSetDir(_) => "tilesetdir",
            SymbolSetDir(_) => "symsetdir",
        }
    }
}

#[derive(Debug, Display)]
enum InvalidConvertSetArgError {
    InvalidConvertArgError(InvalidConvertArgError),
    BinSetInvalidArguments(&'static str),
    TileSetGridsInvalidArguments(&'static str),
}

fn argument_norm_args(arg: &str) -> Result<(&str, Option<&str>), InvalidConvertSetArgError> {
    let args: Vec<&str> = arg.split(':').collect();
    if args.len() > 2 {
        return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too many arguments"))
    } else if args.is_empty() {
        return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too few arguments"))
    }
    let dir = args[0];
    let ident = args.get(1).cloned();
    Ok((dir, ident))
}

fn identify_convert_set_arg(input: &str) -> Result<ConvertSetArg, InvalidConvertSetArgError> {
    if let Some(file_paths) = input.strip_prefix("binset:") {
        let files: Vec<&str> = file_paths.split(':').collect();
        match files.len().cmp(&4) {
            Ordering::Less => return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too few arguments")),
            Ordering::Greater => return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too many arguments")),
            Ordering::Equal => {},
        }
        Ok(ConvertSetArg::BinFileSet { sd_path: files[0], sd_2_path: files[1], hd_path: files[2], hd_2_path: files[3] })

    } else if let Some(path) = input.strip_prefix("binsetnorm:") {
        let (dir, ident) = argument_norm_args(path)?;
        Ok(ConvertSetArg::BinFileSetNorm { dir, ident })

    } else if let Some(file_paths) = input.strip_prefix("tilesetgrids:") {
        let files: Vec<&str> = file_paths.split(':').collect();
        match files.len().cmp(&2) {
            Ordering::Less => return Err(InvalidConvertSetArgError::TileSetGridsInvalidArguments("too few arguments")),
            Ordering::Greater => return Err(InvalidConvertSetArgError::TileSetGridsInvalidArguments("too many arguments")),
            Ordering::Equal => {},
        }
        Ok(ConvertSetArg::TileSetGrids { sd_path: files[0], hd_path: files[1] })

    } else if let Some(path) = input.strip_prefix("tilesetgridsnorm:") {
        let (dir, ident) = argument_norm_args(path)?;
        Ok(ConvertSetArg::TileSetGridsNorm { dir, ident  })

    } else if let Some(path) = input.strip_prefix("tilesetdir:") {
        Ok(ConvertSetArg::TileSetDir(path))

    } else if let Some(path) = input.strip_prefix("symsetdir:") {
        Ok(ConvertSetArg::SymbolSetDir(path))

    } else if let Some((prefix, _)) = input.split_once(':') {
        Err(InvalidConvertSetArgError::InvalidConvertArgError(InvalidConvertArgError::InvalidPrefix(prefix.to_owned())))
    } else {
        Err(InvalidConvertSetArgError::InvalidConvertArgError(InvalidConvertArgError::NoPrefix))
    }
}

#[derive(Debug)]
enum ConvertSetError {
    FromArg(InvalidConvertSetArgError),
    ToArg(InvalidConvertSetArgError),
    InvalidConversion {
        from_prefix: String,
        to_prefix: String
    }
}

impl Error for ConvertSetError {}

impl Display for ConvertSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConvertSetError::*;
        match self {
            FromArg(error) => write!(f, "invalid `from` argument: {}", error),
            ToArg(error) => write!(f, "invalid `to` argument: {}", error),
            InvalidConversion { from_prefix, to_prefix } => write!(f, "invalid conversion from {} to {}", from_prefix, to_prefix),
        }
    }
}

fn convert_tile_set(tile_set: TileSet, to_arg: &ConvertSetArg, options: &ConvertOptions) {
    use ConvertSetArg::*;
    match to_arg {
        BinFileSet { sd_path, sd_2_path, hd_path, hd_2_path } => tile_set.save_to_bin_files(sd_path, sd_2_path, hd_path, hd_2_path).unwrap(),
        BinFileSetNorm { dir, ident } => tile_set.save_to_bin_files_norm(dir, ident).unwrap(),
        TileSetGrids { sd_path, hd_path } => tile_set.save_to_grids(sd_path, hd_path).unwrap(),
        TileSetGridsNorm { dir, ident  } => tile_set.save_to_grids_norm(dir, ident).unwrap(),
        TileSetDir(dir) => tile_set.save_tiles_to_dir(dir).unwrap(),
        SymbolSetDir(dir) => {
            let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
            tile_set.into_symbol_set(&sym_specs).unwrap().save_to_dir(dir).unwrap();
        },
    }
}

fn convert_tile_grid_set(tile_grid_set: TileGridSet, to_arg: &ConvertSetArg, options: &ConvertOptions) {
    convert_tile_set(tile_grid_set.into_tile_set(), to_arg, options)
}


fn convert_set_command(from: &str, to: &str, options: ConvertOptions) -> Result<(), ConvertSetError> {
    let from_arg = identify_convert_set_arg(from).map_err(ConvertSetError::FromArg)?;
    let to_arg = identify_convert_set_arg(to).map_err(ConvertSetError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertSetArg::*;
    match (&from_arg, &to_arg) {
        (BinFileSet{..}, BinFileSet{..}) | (BinFileSetNorm {..}, BinFileSetNorm {..}) | (TileSetGrids{..}, TileSetGrids{..}) |
        (TileSetGridsNorm {..}, TileSetGridsNorm {..}) | (TileSetDir(_), TileSetDir(_)) | (SymbolSetDir(_), SymbolSetDir(_)) =>
            return Err(ConvertSetError::InvalidConversion { from_prefix: from_arg.prefix().to_owned(), to_prefix: to_arg.prefix().to_owned()}),

        (BinFileSet { sd_path, sd_2_path, hd_path, hd_2_path }, to_arg) => {
            let tile_set = bin_file::load_set(sd_path, sd_2_path, hd_path, hd_2_path).unwrap();
            convert_tile_set(tile_set, to_arg, &options)
        },

        (BinFileSetNorm { dir, ident }, to_arg) => {
            let tile_set = bin_file::load_set_norm(dir, ident).unwrap();
            convert_tile_set(tile_set, to_arg, &options)
        },

        (TileSetGrids { sd_path, hd_path }, to_arg) => {
            let tile_grid_set = TileGridSet::load_from_images(sd_path, hd_path).unwrap();
            convert_tile_grid_set(tile_grid_set, to_arg, &options)
        },

        (TileSetGridsNorm { dir, ident }, to_arg) => {
            let tile_grid_set = TileGridSet::load_from_images_norm(dir, ident).unwrap();
            convert_tile_grid_set(tile_grid_set, to_arg, &options)
        },

        (TileSetDir(dir), to_arg) => {
            let tile_set = TileSet::load_from_dir(dir, 512).unwrap();
            convert_tile_set(tile_set, to_arg, &options)
        },

        (SymbolSetDir(dir), to_arg) => {
            let symbol_set = SymbolSet::load_from_dir(dir, 512).unwrap();
            convert_tile_set(symbol_set.into(), to_arg, &options)
        },

    }

    Ok(())
}

#[derive(Debug, Error, Display, From)]
enum CommandError {
    ConvertError(ConvertError),
    ConvertSetError(ConvertSetError),
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result: Result<(), CommandError> = match &cli.command {
        Commands::Convert { from, to, symbol_specs_file } => convert_command(from, to, ConvertOptions { symbol_specs_file }).map_err(CommandError::ConvertError),
        Commands::ConvertSet { from, to, symbol_specs_file } => convert_set_command(from, to, ConvertOptions { symbol_specs_file }).map_err(CommandError::ConvertSetError),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
