use crate::{
    cli::PasswordArgs,
    command::{
        ask_password,
        commons::{collect_split_archives, run_entries},
        Command,
    },
    utils::{env::NamedTempFile, PathPartExt},
};
use clap::{Parser, ValueEnum, ValueHint};
use pna::{Archive, NormalEntry};
use std::path::PathBuf;
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
pub(crate) enum SortBy {
    Name,
    Ctime,
    Mtime,
    Atime,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
pub(crate) enum SortOrder {
    #[value(alias("asc"))]
    Asc,
    #[value(alias("desc"))]
    Desc,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SortKey {
    by: SortBy,
    order: SortOrder,
}

impl Default for SortKey {
    fn default() -> Self {
        Self {
            by: SortBy::Name,
            order: SortOrder::Asc,
        }
    }
}

impl Display for SortKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let by_val = self.by.to_possible_value().expect("No skipped variants");
        let by = by_val.get_name();
        match self.order {
            SortOrder::Asc => f.write_str(by),
            SortOrder::Desc => {
                let order_val = self.order.to_possible_value().expect("No skipped variants");
                let order = order_val.get_name();
                write!(f, "{by}:{order}")
            }
        }
    }
}

impl FromStr for SortKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (by, order) = match s.split_once(':') {
            Some((b, o)) => (b, Some(o)),
            None => (s, None),
        };
        let by = SortBy::from_str(by, true)?;
        let order = match order {
            Some(o) => SortOrder::from_str(o, true)?,
            None => SortOrder::Asc,
        };
        Ok(Self { by, order })
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SortCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(long = "by", num_args = 1.., default_values_t = [SortKey::default()])]
    by: Vec<SortKey>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SortCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        sort_archive(self)
    }
}

fn sort_archive(args: SortCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.archive)?;
    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<std::io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());
    let mut entries = Vec::<NormalEntry<_>>::new();
    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            entries.push(entry?);
            Ok(())
        },
    )?;

    entries.sort_by(|a, b| {
        for key in &args.by {
            let ord = match key.by {
                SortBy::Name => a.header().path().cmp(b.header().path()),
                SortBy::Ctime => a.metadata().created().cmp(&b.metadata().created()),
                SortBy::Mtime => a.metadata().modified().cmp(&b.metadata().modified()),
                SortBy::Atime => a.metadata().accessed().cmp(&b.metadata().accessed()),
            };
            if ord != std::cmp::Ordering::Equal {
                return match key.order {
                    SortOrder::Asc => ord,
                    SortOrder::Desc => ord.reverse(),
                };
            }
        }
        std::cmp::Ordering::Equal
    });

    let mut temp_file =
        NamedTempFile::new(|| args.archive.parent().unwrap_or_else(|| ".".as_ref()))?;
    let mut archive = Archive::write_header(temp_file.as_file_mut())?;
    for entry in entries {
        archive.add_entry(entry)?;
    }
    archive.finalize()?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    let output = args
        .output
        .unwrap_or_else(|| args.archive.remove_part().unwrap());
    temp_file.persist(output)?;

    Ok(())
}
