use crate::utils::setup;
use clap::Parser;
use pna::{Archive, Duration, EntryBuilder, WriteOptions};
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration as StdDuration, SystemTime},
};

fn init_file_archive<P: AsRef<Path>>(path: P, modified: Option<Duration>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let mut builder =
        EntryBuilder::new_file("file.txt".into(), WriteOptions::builder().build()).unwrap();
    if let Some(mtime) = modified {
        builder.modified(mtime);
    }
    builder.write_all(b"from archive").unwrap();
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
}

#[test]
fn keep_older_preserves_existing_files() {
    setup();
    init_file_archive("keep_strategy/keep_older/archive.pna", None);

    let out_dir = PathBuf::from("keep_strategy/keep_older/out");
    fs::create_dir_all(&out_dir).unwrap();
    let target = out_dir.join("file.txt");
    fs::write(&target, b"existing").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "keep_strategy/keep_older/out",
        "--keep-older",
        "keep_strategy/keep_older/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"existing");
}

#[test]
fn keep_newer_preserves_newer_files() {
    setup();
    init_file_archive(
        "keep_strategy/keep_newer/archive.pna",
        Some(Duration::seconds(1)),
    );

    let out_dir = PathBuf::from("keep_strategy/keep_newer/out");
    fs::create_dir_all(&out_dir).unwrap();
    let target = out_dir.join("file.txt");
    fs::write(&target, b"existing").unwrap();

    let file = fs::File::options().write(true).open(&target).unwrap();
    file.set_modified(SystemTime::now() + StdDuration::from_secs(24 * 60 * 60))
        .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "keep_strategy/keep_newer/out",
        "--keep-newer",
        "keep_strategy/keep_newer/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"existing");
}

#[test]
fn keep_newer_overwrites_older_files() {
    setup();
    init_file_archive(
        "keep_strategy/keep_newer_overwrite/archive.pna",
        Some(Duration::seconds(1)),
    );

    let out_dir = PathBuf::from("keep_strategy/keep_newer_overwrite/out");
    fs::create_dir_all(&out_dir).unwrap();
    let target = out_dir.join("file.txt");
    fs::write(&target, b"existing").unwrap();

    let file = fs::File::options().write(true).open(&target).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "keep_strategy/keep_newer_overwrite/out",
        "--keep-newer",
        "keep_strategy/keep_newer_overwrite/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"from archive");
}
