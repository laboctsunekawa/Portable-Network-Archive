use crate::utils::{archive, setup, EmbedExt, TestResources};
use assert_cmd::Command as Cmd;
use std::fs;

fn collect_entry_names(path: impl AsRef<str>) -> Vec<String> {
    let mut names = Vec::new();
    archive::for_each_entry(path.as_ref(), |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names
}

#[test]
fn create_supports_archive_token() {
    setup();
    TestResources::extract_in("deflate.pna", "stdio_archive/create").unwrap();

    let new_file = "stdio_archive/create/new.txt";
    fs::write(new_file, b"fresh data").unwrap();

    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--create",
        "--overwrite",
        "-f",
        "stdio_archive/create/out.pna",
        new_file,
        "@stdio_archive/create/deflate.pna",
    ]);
    cmd.assert().success();

    let mut expected = vec![new_file.to_string()];
    expected.extend(collect_entry_names("stdio_archive/create/deflate.pna"));

    let actual = collect_entry_names("stdio_archive/create/out.pna");
    assert_eq!(actual, expected);
}

#[test]
fn create_supports_stdin_archive_token() {
    setup();
    TestResources::extract_in("deflate.pna", "stdio_archive/stdin").unwrap();

    let new_file = "stdio_archive/stdin/new.txt";
    fs::write(new_file, b"fresh data").unwrap();
    let stdin_bytes = fs::read("stdio_archive/stdin/deflate.pna").unwrap();

    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.write_stdin(stdin_bytes).args([
        "--quiet",
        "experimental",
        "stdio",
        "--create",
        "--overwrite",
        "-f",
        "stdio_archive/stdin/out.pna",
        new_file,
        "@-",
    ]);
    cmd.assert().success();

    let mut expected = vec![new_file.to_string()];
    expected.extend(collect_entry_names("stdio_archive/stdin/deflate.pna"));

    let actual = collect_entry_names("stdio_archive/stdin/out.pna");
    assert_eq!(actual, expected);
}

#[test]
fn append_supports_archive_token() {
    setup();
    TestResources::extract_in("deflate.pna", "stdio_archive/append").unwrap();

    fs::copy(
        "stdio_archive/append/deflate.pna",
        "stdio_archive/append/base.pna",
    )
    .unwrap();
    fs::copy(
        "stdio_archive/append/deflate.pna",
        "stdio_archive/append/extra.pna",
    )
    .unwrap();

    let original = collect_entry_names("stdio_archive/append/base.pna");
    let extra = collect_entry_names("stdio_archive/append/extra.pna");

    let new_file = "stdio_archive/append/new.txt";
    fs::write(new_file, b"fresh data").unwrap();

    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--append",
        "-f",
        "stdio_archive/append/base.pna",
        new_file,
        "@stdio_archive/append/extra.pna",
    ]);
    cmd.assert().success();

    let mut expected = original;
    expected.push(new_file.to_string());
    expected.extend(extra);

    let actual = collect_entry_names("stdio_archive/append/base.pna");
    assert_eq!(actual, expected);
}

#[test]
fn fails_on_non_pna_source() {
    setup();
    fs::create_dir_all("stdio_archive/invalid").unwrap();
    fs::write("stdio_archive/invalid/not_archive.bin", b"plain text").unwrap();

    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--overwrite",
            "-f",
            "stdio_archive/invalid/out.pna",
            "@stdio_archive/invalid/not_archive.bin",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("not a valid PNA"),
        "unexpected stderr: {stderr}"
    );
}
