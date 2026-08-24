#!/usr/bin/env python3
"""Parse bsdtar_test verbose output and enforce the compatibility baseline."""
import json
import os
import re
import sys


# Kept in this script deliberately: the compatibility baseline is CI policy,
# not product configuration. Entries are test names from libarchive v3.8.5's
# bsdtar_test suite and are platform-specific.
_UNIX_EXPECTED_FAILURES = {
    "test_copy",
    "test_crlf_mtree",
    "test_option_C_mtree",
    "test_option_C_upper",
    "test_option_a",
    "test_option_b",
    "test_option_b64encode",
    "test_option_ignore_zeros_mode_c",
    "test_option_j",
    "test_option_lz4",
    "test_option_lzma",
    "test_option_r",
    "test_option_uuencode",
    "test_option_xz",
    "test_option_z",
    "test_option_zstd",
    "test_patterns",
    "test_version",
}
EXPECTED_FAILURES = {
    "ubuntu": _UNIX_EXPECTED_FAILURES,
    "macos": _UNIX_EXPECTED_FAILURES,
    "windows": _UNIX_EXPECTED_FAILURES,
}


def parse_bsdtar_test_output(lines):
    """Parse bsdtar_test -v output lines and return a structured dict.

    Scans line-by-line for execution lines, failure details, and totals.
    Multiple blocks (from ranged test runs) are aggregated into a single result.
    """
    re_exec = re.compile(r"^\s+(\d+):\s+(\S+)\s*$")
    re_fail = re.compile(r"^\s+(\d+):\s+(\S+)\s+\((\d+)\s+failures?\)")
    re_tests_run = re.compile(r"Tests run:\s+(\d+)")
    re_assertions_checked = re.compile(r"Assertions checked:\s+(\d+)")
    re_assertions_failed = re.compile(r"Assertions failed:\s+(\d+)")
    re_skips = re.compile(r"Skips reported:\s+(\d+)")

    tests = []
    failures = {}
    completed_blocks = 0
    sum_skipped = 0
    sum_assertions_checked = 0
    sum_assertions_failed = 0

    for line in lines:
        m = re_exec.match(line)
        if m:
            tests.append((int(m.group(1)), m.group(2)))
            continue

        m = re_fail.match(line)
        if m:
            failures[m.group(2)] = int(m.group(3))
            continue

        if re_tests_run.search(line):
            completed_blocks += 1
            continue

        m = re_skips.search(line)
        if m:
            sum_skipped += int(m.group(1))
            continue

        m = re_assertions_checked.search(line)
        if m:
            sum_assertions_checked += int(m.group(1))
            continue

        m = re_assertions_failed.search(line)
        if m:
            sum_assertions_failed += int(m.group(1))
            continue

    test_results = []
    failed = 0
    for tid, name in tests:
        entry = {"id": tid, "name": name}
        if name in failures:
            entry["status"] = "failed"
            entry["failures"] = failures[name]
            failed += 1
        else:
            entry["status"] = "passed"
        test_results.append(entry)

    total = len(test_results)

    return {
        "completed_blocks": completed_blocks,
        "tests": test_results,
        "summary": {
            "total": total,
            "passed": total - failed,
            "failed": failed,
            "skipped": sum_skipped,
            "assertions_checked": sum_assertions_checked,
            "assertions_failed": sum_assertions_failed,
        },
    }


def validate_expected_failures(result, platform):
    """Require the observed failing-test set to equal the platform XFAIL set."""
    if platform not in EXPECTED_FAILURES:
        print(f"Unknown bsdtar compatibility platform: {platform}", file=sys.stderr)
        return False

    actual = {
        test["name"] for test in result["tests"] if test["status"] == "failed"
    }
    expected = EXPECTED_FAILURES[platform]
    unexpected = sorted(actual - expected)
    xpass = sorted(expected - actual)

    if unexpected:
        print(
            "Unexpected bsdtar compatibility failures: " + ", ".join(unexpected),
            file=sys.stderr,
        )
    if xpass:
        print(
            "Expected failures unexpectedly passed; remove them from XFAIL: "
            + ", ".join(xpass),
            file=sys.stderr,
        )

    return not unexpected and not xpass


def main():
    if len(sys.argv) > 1:
        try:
            f = open(sys.argv[1])
        except FileNotFoundError:
            print(f"Error: file not found: {sys.argv[1]}", file=sys.stderr)
            sys.exit(1)
    else:
        f = sys.stdin

    with f:
        result = parse_bsdtar_test_output(f)

    json.dump(result, sys.stdout, indent=2)
    print()

    platform = os.environ.get("MATRIX_NAME")
    if platform and not validate_expected_failures(result, platform):
        sys.exit(1)


if __name__ == "__main__":
    main()
