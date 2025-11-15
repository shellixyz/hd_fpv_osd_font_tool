#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use std::env::current_exe;
use std::{io::Write, process::exit};

use anyhow::anyhow;
use clap::Parser;
use env_logger::fmt::Color;
use hd_fpv_osd_font_tool::prelude::*;

mod cli;
mod convert;
mod convert_set;
mod man_pages;

#[allow(clippy::wildcard_imports)]
use cli::*;
use convert::convert_command;
use convert_set::convert_set_command;
#[allow(clippy::wildcard_imports)]
use man_pages::*;

fn current_exe_name() -> anyhow::Result<String> {
	let current_exe = current_exe().map_err(|error| anyhow!("failed to get exe name: {error}"))?;
	Ok(current_exe
		.file_name()
		.unwrap()
		.to_str()
		.ok_or_else(|| anyhow!("exe file name contains invalid UTF-8 characters"))?
		.to_string())
}
fn generate_man_pages_command() -> anyhow::Result<()> {
	let current_exe_name = current_exe_name()?;
	generate_exe_man_page(&current_exe_name)?;
	generate_man_page_for_subcommands(&current_exe_name)?;
	Ok(())
}

fn main() {
	let cli = Cli::parse();

	env_logger::builder()
		.format(|buf, record| {
			let level_style = buf.default_level_style(record.level());
			write!(buf, "{:<5}", level_style.value(record.level()))?;
			let mut style = buf.style();
			style.set_color(Color::White).set_bold(true);
			write!(buf, "{}", style.value(" > "))?;
			writeln!(buf, "{}", record.args())
		})
		.parse_filters(cli.log_level().to_string().as_str())
		.init();

	let command_result = match &cli.command {
		Commands::Convert {
			from,
			to,
			symbol_specs_file,
		} => convert_command(from, to, &ConvertOptions { symbol_specs_file }),
		Commands::ConvertSet {
			from,
			to,
			symbol_specs_file,
		} => convert_set_command(from, to, &ConvertOptions { symbol_specs_file }),
		Commands::GenerateManPages => generate_man_pages_command(),
	};

	if let Err(error) = command_result {
		log::error!("{error}");
		exit(1);
	}
}
