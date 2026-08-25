use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

#[test]
fn diff_compare_selects_fields_and_preserves_default_profile() {
    setup();
    let dir = "diff_compare_selection_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "size",
        ])
        .assert()
        .success()
        .stdout("");

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "size,content",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Contents differ"));

    let implicit = cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "default",
        ])
        .assert()
        .code(1)
        .stdout(implicit);
}

#[cfg(unix)]
#[test]
fn diff_compare_mtime_applies_to_directories() {
    use std::time::{Duration, SystemTime};

    setup();
    let dir = "diff_compare_directory_mtime_test";
    let _ = fs::remove_dir_all(dir);
    let subdir = format!("{dir}/subdir");
    fs::create_dir_all(&subdir).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            "--keep-dir",
            &subdir,
        ])
        .assert()
        .success();

    let new_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    filetime::set_file_mtime(&subdir, filetime::FileTime::from_system_time(new_mtime)).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "mtime",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Mod time differs"));
}

#[cfg(not(unix))]
#[test]
fn diff_compare_warns_and_skips_unsupported_field() {
    setup();
    let dir = "diff_compare_unsupported_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "uid",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr(predicate::str::contains(
            "comparison field 'uid' is unsupported on this platform; skipped",
        ));
}
