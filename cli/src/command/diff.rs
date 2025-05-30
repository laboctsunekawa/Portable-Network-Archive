use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{
        ask_password,
        commons::{collect_split_archives, run_process_archive},
        Command,
    },
};
use clap::Parser;
use pna::{DataKind, NormalEntry, ReadOptions};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

#[derive(Parser, Clone, Debug)]
pub(crate) struct DiffCommand {
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
}

impl Command for DiffCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        diff_archive(self)
    }
}

fn diff_archive(args: DiffCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.file.archive)?;
    run_process_archive(
        archives,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            compare_entry(entry, password.as_deref())
        },
    )
}

fn compare_entry<T: AsRef<[u8]>>(entry: NormalEntry<T>, password: Option<&str>) -> io::Result<()> {
    let path = PathBuf::from(entry.header().path().as_path());
    match entry.header().data_kind() {
        DataKind::File => match fs::read(&path) {
            Ok(data) => {
                let mut reader = entry.reader(ReadOptions::with_password(password))?;
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf)?;
                if buf != data {
                    println!("Different file content: {}", path.display());
                }
            }
            Err(_) => {
                println!("Missing file: {}", path.display());
            }
        },
        DataKind::Directory => {
            if !path.is_dir() {
                println!("Missing directory: {}", path.display());
            }
        }
        DataKind::SymbolicLink => match fs::read_link(&path) {
            Ok(link) => {
                let mut reader = entry.reader(ReadOptions::with_password(password))?;
                let mut link_str = String::new();
                reader.read_to_string(&mut link_str)?;
                if link != PathBuf::from(link_str) {
                    println!("Different symlink: {}", path.display());
                }
            }
            Err(_) => {
                println!("Missing symlink: {}", path.display());
            }
        },
        DataKind::HardLink => {
            if !path.exists() {
                println!("Missing hardlink: {}", path.display());
            }
        }
    }
    Ok(())
}
