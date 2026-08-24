#!/usr/bin/env bats

load '../test_helper.bash'

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1
}

teardown_file() {
  popd || exit 1
}

setup() {
  TEST_DIR="test$BATS_TEST_NUMBER"
  mkdir "$TEST_DIR"
  pushd "$TEST_DIR" || exit 1
  printf '1234567890' > file
}

teardown() {
  popd || exit 1
}

create_archive() {
  local archive="$1"
  shift
  pna --log-level error compat bsdtar --unstable -cf "$archive" "$@" file
}

archive_field() {
  local archive="$1"
  local numeric="$2"
  local field="$3"
  local args=(list --unstable --format csv -f "$archive" file)
  if [[ "$numeric" == numeric ]]; then
    args=(list --unstable --format csv --numeric-owner -f "$archive" file)
  fi
  pna --log-level error "${args[@]}" | python3 -c \
    'import csv, sys; print(next(csv.DictReader(sys.stdin))[sys.argv[1]])' "$field"
}

@test "bsdtar --uid/--uname metadata semantics" {
  run create_archive reference.pna
  assert_success
  assert_output ""

  run archive_field reference.pna numeric owner
  assert_success
  reference_uid="$output"

  run create_archive both.pna --uid=65123 --uname=foofoofoo
  assert_success
  assert_output ""
  run archive_field both.pna numeric owner
  assert_output "65123"
  run archive_field both.pna named owner
  assert_output "foofoofoo"

  run create_archive uid.pna --uid=65123
  assert_success
  assert_output ""
  run archive_field uid.pna numeric owner
  assert_output "65123"
  run archive_field uid.pna named owner
  assert_output ""

  run create_archive uname.pna --uname=foofoofoo
  assert_success
  assert_output ""
  run archive_field uname.pna numeric owner
  assert_output "$reference_uid"
  run archive_field uname.pna named owner
  assert_output "foofoofoo"
}

@test "bsdtar --gid/--gname metadata semantics" {
  run create_archive reference.pna
  assert_success
  assert_output ""

  run archive_field reference.pna numeric group
  assert_success
  reference_gid="$output"

  run create_archive both.pna --gid=17 --gname=foofoofoo
  assert_success
  assert_output ""
  run archive_field both.pna numeric group
  assert_output "17"
  run archive_field both.pna named group
  assert_output "foofoofoo"

  run create_archive gname.pna --gname=foofoofoo
  assert_success
  assert_output ""
  run archive_field gname.pna numeric group
  assert_output "$reference_gid"
  run archive_field gname.pna named group
  assert_output "foofoofoo"

  run create_archive gid_empty_name.pna --gid=17 --gname=
  assert_success
  assert_output ""
  run archive_field gid_empty_name.pna numeric group
  assert_output "17"
  run archive_field gid_empty_name.pna named group
  assert_output ""
}

@test "bsdtar --owner metadata semantics" {
  run create_archive reference.pna
  assert_success
  assert_output ""

  run archive_field reference.pna numeric owner
  assert_success
  reference_uid="$output"

  run create_archive numeric.pna --owner=65123
  assert_success
  assert_output ""
  run archive_field numeric.pna numeric owner
  assert_output "65123"
  run archive_field numeric.pna named owner
  assert_output ""

  run create_archive named.pna --owner=foofoofoo
  assert_success
  assert_output ""
  run archive_field named.pna numeric owner
  assert_output "$reference_uid"
  run archive_field named.pna named owner
  assert_output "foofoofoo"

  run create_archive pair.pna --owner=foofoofoo:65123
  assert_success
  assert_output ""
  run archive_field pair.pna numeric owner
  assert_output "65123"
  run archive_field pair.pna named owner
  assert_output "foofoofoo"
}

@test "bsdtar --group metadata semantics" {
  run create_archive reference.pna
  assert_success
  assert_output ""

  run archive_field reference.pna numeric group
  assert_success
  reference_gid="$output"

  run create_archive numeric.pna --group=17
  assert_success
  assert_output ""
  run archive_field numeric.pna numeric group
  assert_output "17"
  run archive_field numeric.pna named group
  assert_output ""

  run create_archive named.pna --group=foofoofoo
  assert_success
  assert_output ""
  run archive_field named.pna numeric group
  assert_output "$reference_gid"
  run archive_field named.pna named group
  assert_output "foofoofoo"

  run create_archive pair.pna --group=foofoofoo:17
  assert_success
  assert_output ""
  run archive_field pair.pna numeric group
  assert_output "17"
  run archive_field pair.pna named group
  assert_output "foofoofoo"
}
