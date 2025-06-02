use crate::utils::{archive, diff::diff, setup, TestResources};
use clap::Parser;
use pna::prelude::*;
use pna::ChunkType;
use portable_network_archive::{cli, command::Command};

#[test]
fn archive_strip_metadata() {
    setup();
    TestResources::extract_in("raw/", "archive_strip_metadata/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_strip_metadata/strip_metadata.pna",
        "--overwrite",
        "archive_strip_metadata/in/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "archive_strip_metadata/strip_metadata.pna",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_strip_metadata/strip_metadata.pna",
        "--overwrite",
        "--out-dir",
        "archive_strip_metadata/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--strip-components",
        "2",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("archive_strip_metadata/in/", "archive_strip_metadata/out/").unwrap();
}

#[test]
fn strip_keep_xattr() {
    setup();
    TestResources::extract_in("raw/", "strip_keep_xattr/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "strip_keep_xattr/archive.pna",
        "--overwrite",
        "strip_keep_xattr/in/",
        "--keep-xattr",
    ])
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.write_stdin(concat!(
        "# file: strip_keep_xattr/in/raw/empty.txt\n",
        "user.name=\"pna\"\n",
    ));
    cmd.args([
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "strip_keep_xattr/archive.pna",
        "--restore",
        "-",
    ])
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "strip_keep_xattr/archive.pna",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("strip_keep_xattr/archive.pna", |entry| {
        if entry.header().path() == "raw/empty.txt" {
            assert!(entry
                .xattrs()
                .iter()
                .any(|x| x.name() == "user.name" && x.value() == b"pna"));
        }
    })
    .unwrap();
}

#[test]
fn strip_remove_xattr() {
    setup();
    TestResources::extract_in("raw/", "strip_remove_xattr/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "strip_remove_xattr/archive.pna",
        "--overwrite",
        "strip_remove_xattr/in/",
        "--keep-xattr",
    ])
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.write_stdin(concat!(
        "# file: strip_remove_xattr/in/raw/empty.txt\n",
        "user.name=\"pna\"\n",
    ));
    cmd.args([
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "strip_remove_xattr/archive.pna",
        "--restore",
        "-",
    ])
    .unwrap();

    cli::Cli::try_parse_from(["pna", "--quiet", "strip", "strip_remove_xattr/archive.pna"])
        .unwrap()
        .execute()
        .unwrap();

    archive::for_each_entry("strip_remove_xattr/archive.pna", |entry| {
        assert!(entry.xattrs().is_empty());
    })
    .unwrap();
}

#[test]
fn strip_keep_acl_and_private() {
    setup();
    TestResources::extract_in("mixed_acl.pna", "strip_keep_acl/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "strip_keep_acl/mixed_acl.pna",
        "--keep-acl",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut count = 0;
    archive::for_each_entry("strip_keep_acl/mixed_acl.pna", |entry| {
        count += entry
            .extra_chunks()
            .iter()
            .filter(|c| {
                let ty = c.ty();
                ty == unsafe { ChunkType::from_unchecked(*b"faCl") }
                    || ty == unsafe { ChunkType::from_unchecked(*b"faCe") }
            })
            .count();
    })
    .unwrap();
    assert!(count > 0);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "strip_keep_acl/mixed_acl.pna",
        "--keep-private",
        "faCl",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut has_facl = false;
    let mut has_face = false;
    archive::for_each_entry("strip_keep_acl/mixed_acl.pna", |entry| {
        for c in entry.extra_chunks() {
            let ty = c.ty();
            if ty == unsafe { ChunkType::from_unchecked(*b"faCl") } {
                has_facl = true;
            }
            if ty == unsafe { ChunkType::from_unchecked(*b"faCe") } {
                has_face = true;
            }
        }
    })
    .unwrap();
    assert!(has_facl);
    assert!(!has_face);
}

#[test]
fn strip_output_option() {
    setup();
    TestResources::extract_in("zstd.pna", "strip_output/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "strip_output/zstd.pna",
        "--output",
        "strip_output/out.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(std::path::Path::new("strip_output/out.pna").exists());
    assert!(std::path::Path::new("strip_output/zstd.pna").exists());
}
