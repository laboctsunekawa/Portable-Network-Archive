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
_WINDOWS_EXPECTED_FAILURES = (
    _UNIX_EXPECTED_FAILURES
    - {
        "test_copy",
        "test_option_ignore_zeros_mode_c",
    }
    | {
        "test_empty_mtree",
        "test_list_item",
        "test_option_s",
    }
)
EXPECTED_FAILURES = {
    "ubuntu": _UNIX_EXPECTED_FAILURES,
    "macos": _UNIX_EXPECTED_FAILURES,
    "windows": _WINDOWS_EXPECTED_FAILURES,
}


def parse_bsdtar_test_output(lines):
    """Parse bsdtar_test -v output lines and return a structured dict.

    Scans line-by-line for execution lines, failure details, skips, and totals.
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
    skipped_tests = set()
    current_test = None
    completed_blocks = 0
    sum_skip_reports = 0
    sum_assertions_checked = 0
    sum_assertions_failed = 0

    for line in lines:
        m = re_exec.match(line)
        if m:
            current_test = m.group(2)
            tests.append((int(m.group(1)), current_test))
            continue

        m = re_fail.match(line)
        if m:
            failures[m.group(2)] = int(m.group(3))
            continue

        if "SKIPPING:" in line and current_test is not None:
            skipped_tests.add(current_test)
            continue

        if re_tests_run.search(line):
            current_test = None
            completed_blocks += 1
            continue

        m = re_skips.search(line)
        if m:
            sum_skip_reports += int(m.group(1))
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
    passed = 0
    failed = 0
    skipped = 0
    for tid, name in tests:
        entry = {"id": tid, "name": name}
        if name in failures:
            entry["status"] = "failed"
            entry["failures"] = failures[name]
            failed += 1
        elif name in skipped_tests:
            entry["status"] = "skipped"
            skipped += 1
        else:
            entry["status"] = "passed"
            passed += 1
        test_results.append(entry)

    total = len(test_results)

    return {
        "completed_blocks": completed_blocks,
        "tests": test_results,
        "summary": {
            "total": total,
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
            "skip_reports": sum_skip_reports,
            "assertions_checked": sum_assertions_checked,
            "assertions_failed": sum_assertions_failed,
        },
    }


def compare_expected_failures(result, platform):
    """Return the exact set difference between observed failures and XFAIL."""
    if platform not in EXPECTED_FAILURES:
        return {
            "platform": platform,
            "error": f"Unknown bsdtar compatibility platform: {platform}",
            "unexpected_failures": [],
            "unexpected_passes": [],
            "matches": False,
        }

    actual = {
        test["name"] for test in result["tests"] if test["status"] == "failed"
    }
    expected = EXPECTED_FAILURES[platform]
    unexpected = sorted(actual - expected)
    xpass = sorted(expected - actual)

    return {
        "platform": platform,
        "unexpected_failures": unexpected,
        "unexpected_passes": xpass,
        "matches": not unexpected and not xpass,
    }


def report_baseline_mismatch(comparison):
    """Make an XFAIL mismatch visible in logs and the GitHub job summary."""
    error = comparison.get("error")
    unexpected = comparison["unexpected_failures"]
    xpass = comparison["unexpected_passes"]

    if error:
        print(error, file=sys.stderr)
    if unexpected:
        print("Unexpected bsdtar compatibility failures:", file=sys.stderr)
        for name in unexpected:
            print(f"  {name}", file=sys.stderr)
    if xpass:
        print("Expected failures did not fail; remove or review XFAIL:", file=sys.stderr)
        for name in xpass:
            print(f"  {name}", file=sys.stderr)

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary_path or comparison["matches"]:
        return

    with open(summary_path, "a", encoding="utf-8") as summary:
        summary.write(
            f"### XFAIL baseline mismatch ({comparison['platform']})\n\n"
        )
        if error:
            summary.write(f"> **Error:** {error}\n\n")
        if unexpected:
            summary.write("**Unexpected failures**\n\n")
            for name in unexpected:
                summary.write(f"- `{name}`\n")
            summary.write("\n")
        if xpass:
            summary.write("**Expected failures that did not fail (stale XFAIL)**\n\n")
            for name in xpass:
                summary.write(f"- `{name}`\n")
            summary.write("\n")


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

    platform = os.environ.get("MATRIX_NAME")
    comparison = None
    if platform:
        comparison = compare_expected_failures(result, platform)
        result["xfail"] = comparison

    json.dump(result, sys.stdout, indent=2)
    print()

    if comparison and not comparison["matches"]:
        report_baseline_mismatch(comparison)
        sys.exit(1)


if __name__ == "__main__":
    main()
