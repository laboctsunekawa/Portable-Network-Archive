use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn verify_archive() {
    setup();
    TestResources::extract_in("store.pna", "verify_archive/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "verify_archive/store.pna",
        "--verify",
    ])
    .unwrap()
    .execute()
    .unwrap();
}

#[test]
fn verify_archive_with_password() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "verify_archive_with_password/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "verify_archive_with_password/zstd_aes_ctr.pna",
        "--verify",
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();
}
