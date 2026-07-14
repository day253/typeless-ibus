#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
spec_file=$project_dir/packaging/typeless-ibus.spec
rpm_topdir=$project_dir/target/rpm

version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_dir/Cargo.toml" | sed -n '1p')
spec_version=$(sed -n 's/^Version:[[:space:]]*//p' "$spec_file" | sed -n '1p')

if [ -z "$version" ] || [ "$version" != "$spec_version" ]; then
  echo "Cargo.toml version ($version) does not match RPM spec version ($spec_version)." >&2
  exit 1
fi

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/typeless-ibus-rpm.XXXXXX")
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM

source_dir=$work_dir/typeless-ibus-$version
rm -rf "$rpm_topdir"
mkdir -p "$source_dir" "$rpm_topdir/BUILD" "$rpm_topdir/BUILDROOT" \
  "$rpm_topdir/RPMS" "$rpm_topdir/SOURCES" "$rpm_topdir/SPECS" "$rpm_topdir/SRPMS"

(
  cd "$project_dir"
  tar \
    --exclude=.cargo \
    --exclude=.git \
    --exclude=result \
    --exclude=target \
    --exclude=vendor \
    -cf - .
) | (
  cd "$source_dir"
  tar -xf -
)

mkdir -p "$source_dir/.cargo"
(
  cd "$source_dir"
  cargo vendor --locked vendor > .cargo/config.toml
)

tar -C "$work_dir" -czf \
  "$rpm_topdir/SOURCES/typeless-ibus-$version.tar.gz" \
  "typeless-ibus-$version"
cp "$spec_file" "$rpm_topdir/SPECS/typeless-ibus.spec"

rpmbuild --define "_topdir $rpm_topdir" -ba "$rpm_topdir/SPECS/typeless-ibus.spec"

find "$rpm_topdir/RPMS" "$rpm_topdir/SRPMS" -type f -name '*.rpm' -print
