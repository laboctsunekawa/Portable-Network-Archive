use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{
        Command, ExitCodeError, ask_password,
        core::{SplitArchiveReader, cmp_at_stored_precision, collect_split_archives},
    },
    utils::{BsdGlobMatcher, io::streams_equal},
};
use clap::{Parser, ValueEnum};
use pna::prelude::SystemTimeDurationExt;
use pna::{DataKind, EntryContent, NormalEntry, ReadOptions};
use same_file::is_same_file;
use std::cmp::Ordering;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::SystemTime;
use std::{fmt, fs, io, path::Path};

#[derive(Parser, Clone, Debug)]
pub(crate) struct DiffCommand {
    #[command(flatten)]
    file: FileArgs,
    #[command(flatten)]
    password: PasswordArgs,
    #[arg(
        long,
        conflicts_with = "compare",
        help = "Compare directory mtime and ownership (by default, only mode is compared for directories)"
    )]
    full_compare: bool,
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Compare only selected fields; repeat or separate values with commas"
    )]
    compare: Vec<CompareField>,
    #[arg(
        long,
        default_value = "plain",
        help = "Output format [unstable: jsonl]",
        long_help = "Output format. plain: tar-style text. jsonl: one JSON Lines record per difference with fields `path`, `kind` (one of: missing, size, content, mode, mtime, uid, gid, type, symlink, hardlink) and, for kind=hardlink only, `target` (the stored link target). mode/uid/gid comparisons are Unix-only."
    )]
    format: Format,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
enum CompareField {
    Default,
    Size,
    Content,
    Mtime,
    Mode,
    Uid,
    Gid,
    Symlink,
    Hardlink,
}

impl CompareField {
    #[inline]
    const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Size => "size",
            Self::Content => "content",
            Self::Mtime => "mtime",
            Self::Mode => "mode",
            Self::Uid => "uid",
            Self::Gid => "gid",
            Self::Symlink => "symlink",
            Self::Hardlink => "hardlink",
        }
    }

    #[cfg(unix)]
    #[inline]
    const fn is_supported(self) -> bool {
        true
    }

    #[cfg(not(unix))]
    #[inline]
    const fn is_supported(self) -> bool {
        !matches!(self, Self::Mode | Self::Uid | Self::Gid)
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "lower")]
enum Format {
    Plain,
    JsonL,
}

impl Format {
    /// Returns true if this format is unstable and requires --unstable flag
    #[inline]
    const fn is_unstable(self) -> bool {
        matches!(self, Self::JsonL)
    }
}

impl fmt::Display for Format {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_possible_value().unwrap().get_name())
    }
}

impl Command for DiffCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        match diff_archive(ctx, self) {
            Ok(0) => Ok(()),
            Ok(_) => Err(ExitCodeError::silent(1).into()),
            Err(err) => Err(ExitCodeError::with_source(2, err).into()),
        }
    }
}

#[hooq::hooq(anyhow)]
fn diff_archive(ctx: &crate::cli::GlobalContext, args: DiffCommand) -> anyhow::Result<usize> {
    if args.format.is_unstable() && !ctx.unstable() {
        anyhow::bail!(
            "The '--format {}' option is unstable and requires --unstable flag",
            args.format
        );
    }
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.file.archive)?;
    let options = CompareOptions::new(args.compare, args.full_compare, args.format);

    let mut globs = BsdGlobMatcher::new(args.file.files.iter().map(|s| s.as_str()));
    let filter_enabled = !globs.is_empty();

    let read_options = ReadOptions::with_password(password.as_deref());
    let mut source = SplitArchiveReader::new(archives)?;
    let mut diff_count = 0usize;
    source.for_each_entry(
        &read_options,
        #[hooq::skip_all]
        |entry| {
            let entry = entry?;
            let path = entry.header().path();

            if filter_enabled && !globs.matches(path) {
                return Ok(());
            }

            diff_count += compare_entry(entry, &read_options, &options)?;
            Ok(())
        },
    )?;

    globs.ensure_all_matched()?;

    Ok(diff_count)
}

/// Difference types detected during archive-filesystem comparison.
/// Message format follows tar --diff for compatibility.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind")]
enum DiffKind {
    /// File/directory does not exist on filesystem
    #[serde(rename = "missing")]
    Missing,
    /// File size differs
    #[serde(rename = "size")]
    SizeDiffers,
    /// File contents differ (same size)
    #[serde(rename = "content")]
    ContentsDiffer,
    /// Permission mode differs
    #[cfg(unix)]
    #[serde(rename = "mode")]
    ModeDiffers,
    /// Modification time differs
    #[serde(rename = "mtime")]
    MtimeDiffers,
    /// User ID differs
    #[cfg(unix)]
    #[serde(rename = "uid")]
    UidDiffers,
    /// Group ID differs
    #[cfg(unix)]
    #[serde(rename = "gid")]
    GidDiffers,
    /// File type differs (e.g., file vs directory)
    #[serde(rename = "type")]
    TypeMismatch,
    /// Symbolic link target differs
    #[serde(rename = "symlink")]
    SymlinkDiffers,
    /// Hardlink relationship broken
    #[serde(rename = "hardlink")]
    NotLinked { target: String },
}

impl DiffKind {
    /// Returns a displayable message for this difference.
    fn display<'a>(&'a self, path: &'a str) -> DiffMessage<'a> {
        DiffMessage { kind: self, path }
    }
}

#[derive(serde::Serialize)]
struct DiffRecord<'a> {
    path: &'a str,
    #[serde(flatten)]
    kind: &'a DiffKind,
}

fn report(kind: &DiffKind, path: &str, format: Format) {
    match format {
        Format::Plain => println!("{}", kind.display(path)),
        Format::JsonL => println!(
            "{}",
            serde_json::to_string(&DiffRecord { path, kind }).unwrap()
        ),
    }
}

/// A tar-compatible difference message that implements `Display`.
struct DiffMessage<'a> {
    kind: &'a DiffKind,
    path: &'a str,
}

impl fmt::Display for DiffMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DiffKind::Missing => {
                write!(
                    f,
                    "{}: Warning: Cannot stat: No such file or directory",
                    self.path
                )
            }
            DiffKind::SizeDiffers => write!(f, "{}: Size differs", self.path),
            DiffKind::ContentsDiffer => write!(f, "{}: Contents differ", self.path),
            #[cfg(unix)]
            DiffKind::ModeDiffers => write!(f, "{}: Mode differs", self.path),
            DiffKind::MtimeDiffers => write!(f, "{}: Mod time differs", self.path),
            #[cfg(unix)]
            DiffKind::UidDiffers => write!(f, "{}: Uid differs", self.path),
            #[cfg(unix)]
            DiffKind::GidDiffers => write!(f, "{}: Gid differs", self.path),
            DiffKind::TypeMismatch => write!(f, "{}: File type differs", self.path),
            DiffKind::SymlinkDiffers => write!(f, "{}: Symlink differs", self.path),
            DiffKind::NotLinked { target } => write!(f, "{}: Not linked to {target}", self.path),
        }
    }
}

/// Options controlling what aspects to compare.
#[derive(Clone, Debug)]
struct CompareOptions {
    default_profile: bool,
    fields: Vec<CompareField>,
    #[cfg_attr(not(unix), allow(dead_code))]
    full_compare: bool,
    format: Format,
}

impl CompareOptions {
    fn new(compare: Vec<CompareField>, full_compare: bool, format: Format) -> Self {
        let default_profile = compare.is_empty() || compare.contains(&CompareField::Default);
        let mut fields = Vec::new();
        for field in compare {
            if field == CompareField::Default || fields.contains(&field) {
                continue;
            }
            if !field.is_supported() {
                log::warn!(
                    "comparison field '{}' is unsupported on this platform; skipped",
                    field.name()
                );
                continue;
            }
            fields.push(field);
        }
        Self {
            default_profile,
            fields,
            full_compare,
            format,
        }
    }

    #[inline]
    fn explicitly_enabled(&self, field: CompareField) -> bool {
        self.fields.contains(&field)
    }

    #[inline]
    fn enabled(&self, field: CompareField, data_kind: DataKind) -> bool {
        self.explicitly_enabled(field) || self.default_enabled(field, data_kind)
    }

    fn default_enabled(&self, field: CompareField, data_kind: DataKind) -> bool {
        if !self.default_profile {
            return false;
        }
        match field {
            CompareField::Size | CompareField::Content => data_kind == DataKind::FILE,
            CompareField::Symlink => data_kind == DataKind::SYMBOLIC_LINK,
            CompareField::Hardlink => data_kind == DataKind::HARD_LINK,
            CompareField::Mtime => {
                #[cfg(unix)]
                {
                    data_kind == DataKind::FILE
                        || (data_kind == DataKind::DIRECTORY && self.full_compare)
                }
                #[cfg(not(unix))]
                {
                    false
                }
            }
            CompareField::Mode => {
                #[cfg(unix)]
                {
                    matches!(data_kind, DataKind::FILE | DataKind::DIRECTORY)
                }
                #[cfg(not(unix))]
                {
                    false
                }
            }
            CompareField::Uid | CompareField::Gid => {
                #[cfg(unix)]
                {
                    data_kind == DataKind::FILE
                        || (data_kind == DataKind::DIRECTORY && self.full_compare)
                }
                #[cfg(not(unix))]
                {
                    false
                }
            }
            CompareField::Default => false,
        }
    }
}

fn matches_at_stored_precision(archived: pna::Duration, fs: SystemTime) -> bool {
    fs.try_duration_since_unix_epoch_signed()
        .is_ok_and(|fs| cmp_at_stored_precision(archived, fs) == Ordering::Equal)
}

fn compare_metadata<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    fs_meta: &fs::Metadata,
    data_kind: DataKind,
    options: &CompareOptions,
) -> io::Result<Vec<DiffKind>> {
    let mut diffs = Vec::new();
    #[cfg(unix)]
    let ownership = crate::ext::ResolvedOwnership::from_metadata(entry.metadata());

    #[cfg(unix)]
    if options.enabled(CompareField::Mode, data_kind)
        && let Some(mode) = ownership.mode
    {
        let archive_mode = mode & 0o7777;
        let fs_mode = (fs_meta.permissions().mode() & 0o7777) as u16;
        if archive_mode != fs_mode {
            diffs.push(DiffKind::ModeDiffers);
        }
    }

    if options.enabled(CompareField::Mtime, data_kind)
        && let Some(archive_mtime) = entry.metadata().modified()
    {
        match fs_meta.modified() {
            Ok(fs_mtime) if !matches_at_stored_precision(archive_mtime, fs_mtime) => {
                diffs.push(DiffKind::MtimeDiffers);
            }
            Ok(_) => {}
            Err(e) if options.explicitly_enabled(CompareField::Mtime) => return Err(e),
            Err(_) => {}
        }
    }

    #[cfg(unix)]
    if options.enabled(CompareField::Uid, data_kind)
        && let Some(uid) = ownership.uid
        && uid != fs_meta.uid() as u64
    {
        diffs.push(DiffKind::UidDiffers);
    }

    #[cfg(unix)]
    if options.enabled(CompareField::Gid, data_kind)
        && let Some(gid) = ownership.gid
        && gid != fs_meta.gid() as u64
    {
        diffs.push(DiffKind::GidDiffers);
    }

    Ok(diffs)
}

fn compare_entry<T: AsRef<[u8]>>(
    entry: NormalEntry<T>,
    read_options: &ReadOptions,
    options: &CompareOptions,
) -> io::Result<usize> {
    let data_kind = entry.header().data_kind();
    let path = entry.header().path();
    let path_str = path.as_str();
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            report(&DiffKind::Missing, path_str, options.format);
            return Ok(1);
        }
        Err(e) => return Err(e),
    };

    let type_matches = match data_kind {
        DataKind::FILE => meta.is_file(),
        DataKind::DIRECTORY => meta.is_dir(),
        DataKind::SYMBOLIC_LINK => meta.is_symlink(),
        DataKind::HARD_LINK => meta.is_file(),
        _ => false,
    };
    if !type_matches {
        report(&DiffKind::TypeMismatch, path_str, options.format);
        return Ok(1);
    }

    let meta_diffs = compare_metadata(&entry, &meta, data_kind, options)?;
    let mut diff_count = meta_diffs.len();
    for diff in &meta_diffs {
        report(diff, path_str, options.format);
    }

    match data_kind {
        DataKind::FILE => {
            let compare_size = options.enabled(CompareField::Size, data_kind);
            let compare_content = options.enabled(CompareField::Content, data_kind);
            if compare_size || compare_content {
                let fs_size = meta.len();
                let archive_size = entry.metadata().raw_file_size();
                if archive_size.is_some_and(|s| s != fs_size as u128) {
                    if compare_size {
                        report(&DiffKind::SizeDiffers, path_str, options.format);
                    } else {
                        report(&DiffKind::ContentsDiffer, path_str, options.format);
                    }
                    diff_count += 1;
                } else if compare_content {
                    let fs_file = fs::File::open(path)?;
                    let archive_reader = entry.reader(read_options)?;
                    if !streams_equal(fs_file, archive_reader)? {
                        report(&DiffKind::ContentsDiffer, path_str, options.format);
                        diff_count += 1;
                    }
                }
            }
        }
        DataKind::DIRECTORY => {}
        DataKind::SYMBOLIC_LINK if options.enabled(CompareField::Symlink, data_kind) => {
            let link = fs::read_link(path)?;
            let EntryContent::SymbolicLink(stored) = entry.content(read_options)? else {
                unreachable!("data_kind() returned SymbolicLink");
            };
            if link.as_path() != Path::new(stored.as_str()) {
                report(&DiffKind::SymlinkDiffers, path_str, options.format);
                diff_count += 1;
            }
        }
        DataKind::HARD_LINK if options.enabled(CompareField::Hardlink, data_kind) => {
            let EntryContent::HardLink(stored) = entry.content(read_options)? else {
                unreachable!("data_kind() returned HardLink");
            };
            match is_same_file(path, stored.as_str()) {
                Ok(true) => (),
                Ok(false) => {
                    report(
                        &DiffKind::NotLinked {
                            target: stored.to_string(),
                        },
                        path_str,
                        options.format,
                    );
                    diff_count += 1;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    report(&DiffKind::Missing, path_str, options.format);
                    diff_count += 1;
                }
                Err(e) => return Err(e),
            }
        }
        DataKind::SYMBOLIC_LINK | DataKind::HARD_LINK => {}
        _ => unreachable!("type compatibility was checked above"),
    }
    Ok(diff_count)
}
