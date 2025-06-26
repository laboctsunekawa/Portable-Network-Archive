#!/usr/bin/env bash
set -e
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
mkdir "$TMPDIR/src"
echo first > "$TMPDIR/src/a.txt"
echo second > "$TMPDIR/src/b.txt"

# First backup
pna create "$TMPDIR/pna1.pna" --overwrite "$TMPDIR/src" --listed-incremental "$TMPDIR/pna.snar" --quiet
 tar --listed-incremental="$TMPDIR/tar.snar" -cf "$TMPDIR/tar1.tar" -C "$TMPDIR/src" .

pna list "$TMPDIR/pna1.pna" --quiet | sort > "$TMPDIR/pna.lst"
tar -tf "$TMPDIR/tar1.tar" | sort > "$TMPDIR/tar.lst"
diff "$TMPDIR/pna.lst" "$TMPDIR/tar.lst"

# Modify one file and add another
sleep 1
echo updated >> "$TMPDIR/src/a.txt"
echo third > "$TMPDIR/src/c.txt"

pna create "$TMPDIR/pna2.pna" --overwrite "$TMPDIR/src" --listed-incremental "$TMPDIR/pna.snar" --quiet
 tar --listed-incremental="$TMPDIR/tar.snar" -cf "$TMPDIR/tar2.tar" -C "$TMPDIR/src" .

pna list "$TMPDIR/pna2.pna" --quiet | sort > "$TMPDIR/pna2.lst"
tar -tf "$TMPDIR/tar2.tar" | sort > "$TMPDIR/tar2.lst"
diff "$TMPDIR/pna2.lst" "$TMPDIR/tar2.lst"
