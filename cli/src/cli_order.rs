use crate::{
    cli::{Cli, Commands},
    command::compat::CompatCommands,
};
use clap::{CommandFactory, FromArgMatches};
use std::ffi::OsString;

/// Parses the CLI while preserving the relative order of bsdtar `-C` options
/// and file operands, which derive parsing otherwise stores in separate fields.
#[doc(hidden)]
pub fn parse_cli_from(args: Vec<OsString>) -> Cli {
    let matches = Cli::command().get_matches_from(args);
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());

    let Some(compat_matches) = matches.subcommand_matches("compat") else {
        return cli;
    };
    let Some(bsdtar_matches) = compat_matches.subcommand_matches("bsdtar") else {
        return cli;
    };
    let Commands::Compat(compat) = &mut cli.commands else {
        return cli;
    };
    let CompatCommands::Bsdtar(bsdtar) = &mut compat.command;
    bsdtar.capture_operand_order(bsdtar_matches);

    cli
}
