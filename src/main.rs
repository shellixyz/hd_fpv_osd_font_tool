
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::process::exit;

use clap::{Parser, Subcommand};

use hd_fpv_osd_font_tool::osd::{
    bin_file::BinFileReader, SaveTilesToDir, tile::grid::TileGrid, SaveTilesToBinFile,
    tile::containers::StandardSizeArray
};

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
    /// Takes two arguments: the source and destination collection types and path in the form of type:path
    ///
    /// Valid types are:{n}
    ///     * bin: for raw RGBA files usually named with the .bin extension{n}
    ///     * tilegrid: image with the tiles arranged in a grid{n} (only PNG supported){n}
    ///     * tiledir: directory with each tile named with its index starting at 000 and ending with 255
    ///
    /// Example: extracting the tiles from a bin file to individual files in the tiles directory:{n}
    ///     `convert bin:font.bin tiledir:tiles`
    Convert {
        /// source collection type and path in the format type:path
        from: String,

        /// destination collection type and path in the format type:path
        to: String
    }
}

#[derive(Debug)]
enum InvalidConvertArgError<'a> {
    InvalidPrefix(&'a str),
    InvalidImageFileExtension {
        path: &'a str,
        extension: Option<&'a str>
    },
    InvalidPath(&'a str),
    NoPrefix
}

impl<'a> Error for InvalidConvertArgError<'a> {}

impl<'a> Display for InvalidConvertArgError<'a> {
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
    TileDir(&'a str)
}

impl<'a> ConvertArg<'a> {

    fn prefix(&self) -> &'static str {
        match self {
            ConvertArg::BinFile(_) => "bin",
            ConvertArg::TileGrid(_) => "tilegrid",
            ConvertArg::TileDir(_) => "tiledir",
        }
    }
}

fn check_arg_image_file_extension(path: &str) -> Result<(), InvalidConvertArgError> {
    match Path::extension(Path::new(path)) {
        Some(os_str) => match os_str.to_str() {
            Some("png") => Ok(()),
            extension @ Some(_) => Err(InvalidConvertArgError::InvalidImageFileExtension { path, extension }),
            None => Err(InvalidConvertArgError::InvalidPath(path))
        },
        None => Err(InvalidConvertArgError::InvalidImageFileExtension { path, extension: None })
    }
}

fn identify_convert_arg(input: &str) -> Result<ConvertArg, InvalidConvertArgError> {
    if let Some(path) = input.strip_prefix("bin:") {
        Ok(ConvertArg::BinFile(path))
    } else if let Some(path) = input.strip_prefix("tilegrid:") {
        Ok(ConvertArg::TileGrid(path))
    } else if let Some(path) = input.strip_prefix("tiledir:") {
        Ok(ConvertArg::TileDir(path))
    } else if let Some((prefix, _)) = input.split_once(':') {
        Err(InvalidConvertArgError::InvalidPrefix(prefix))
    } else {
        Err(InvalidConvertArgError::NoPrefix)
    }
}

#[derive(Debug)]
enum ConvertError<'a> {
    FromArg(InvalidConvertArgError<'a>),
    ToArg(InvalidConvertArgError<'a>),
    InvalidConversion {
        from_prefix: &'a str,
        to_prefix: &'a str
    }
}

impl<'a> Error for ConvertError<'a> {}

impl<'a> Display for ConvertError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConvertError::*;
        match self {
            FromArg(error) => write!(f, "invalid `from` argument: {}", error),
            ToArg(error) => write!(f, "invalid `to` argument: {}", error),
            InvalidConversion { from_prefix, to_prefix } => write!(f, "invalid conversion from {} to {}", from_prefix, to_prefix),
        }
    }
}

fn convert_command<'a>(from: &'a String, to: &'a String) -> Result<(), ConvertError<'a>> {
    let from_arg = identify_convert_arg(from).map_err(ConvertError::FromArg)?;
    let to_arg = identify_convert_arg(to).map_err(ConvertError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertArg::*;
    match (&from_arg, &to_arg) {
        (BinFile(_), BinFile(_)) | (TileGrid(_), TileGrid(_)) | (TileDir(_), TileDir(_)) =>
            return Err(ConvertError::InvalidConversion { from_prefix: from_arg.prefix(), to_prefix: to_arg.prefix()}),

        (BinFile(from_path), to_arg) => {
            let mut bin_file = BinFileReader::open(from_path).unwrap();
            match to_arg {
                TileGrid(to_path) => {
                    check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
                    bin_file.tile_grid().unwrap().image().save(to_path).unwrap()
                },
                TileDir(to_path) => bin_file.tile_array().unwrap().save_tiles_to_dir(to_path).unwrap(),
                _ => unreachable!()
            }
        },

        (TileGrid(from_path), to_arg) => {
            check_arg_image_file_extension(from_path).map_err(ConvertError::FromArg)?;
            let tile_grid = crate::TileGrid::load_from_image(from_path).unwrap();
            match to_arg {
                BinFile(to_path) => tile_grid.save_tiles_to_bin_file(to_path).unwrap(),
                TileDir(to_path) => tile_grid.save_tiles_to_dir(to_path).unwrap(),
                _ => unreachable!()
            }
        },

        (TileDir(from_path), to_arg) => {
            let tile_array = StandardSizeArray::load_from_dir(from_path).unwrap();
            match to_arg {
                BinFile(to_path) => tile_array.save_tiles_to_bin_file(to_path).unwrap(),
                TileGrid(to_path) => {
                    check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
                    tile_array.into_grid().image().save(to_path).unwrap()
                },
                _ => unreachable!()
            }
        },
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {
        Commands::Convert { from, to } => convert_command(from, to)
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
