use crate::{
    cli::PasswordArgs,
    command::{
        ask_password,
        commons::{collect_split_archives, run_entries},
        Command,
    },
    utils::PathPartExt,
};
use clap::{Parser, ValueEnum, ValueHint};
use pna::{Archive, NormalEntry};
use std::{fs, io, path::PathBuf};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
pub(crate) enum SortBy {
    Name,
    Ctime,
    Mtime,
    Atime,
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SortCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(long = "by", value_enum, num_args = 1.., default_values_t = [SortBy::Name])]
    by: Vec<SortBy>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SortCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        sort_archive(self)
    }
}

fn sort_archive(args: SortCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.archive)?;
    let mut entries = Vec::<NormalEntry<Vec<u8>>>::new();
    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            #[allow(clippy::useless_conversion)]
            {
                entries.push(entry?.into());
            }
            Ok(())
        },
    )?;

    for by in args.by.iter().rev() {
        match by {
            SortBy::Name => entries.sort_by(|a, b| a.header().path().cmp(b.header().path())),
            SortBy::Ctime | SortBy::Mtime | SortBy::Atime => todo!(),
        }
    }

    let output = args
        .output
        .unwrap_or_else(|| args.archive.remove_part().unwrap());
    let outfile = fs::File::create(&output)?;
    let mut archive = Archive::write_header(outfile)?;
    for entry in entries {
        archive.add_entry(entry)?;
    }
    archive.finalize()?;
    Ok(())
}
