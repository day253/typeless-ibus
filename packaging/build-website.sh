#!/bin/sh
set -eu

project_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
source_dir=$project_dir/website
output_dir=${1:-$project_dir/target/pages}
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_dir/Cargo.toml" | sed -n '1p')

if [ -z "$version" ]; then
  echo "Unable to read the package version from Cargo.toml." >&2
  exit 1
fi

case "$version" in
  *[!0-9A-Za-z.+-]*)
    echo "Cargo.toml contains an invalid package version: $version" >&2
    exit 1
    ;;
esac

rm -rf "$output_dir"
mkdir -p "$output_dir"
cp -R "$source_dir"/. "$output_dir"/

find "$output_dir" -type f \( -name '*.html' -o -name '*.js' \) | while IFS= read -r file; do
  sed "s/@TYPELESS_VERSION@/$version/g" "$file" > "$file.tmp"
  mv "$file.tmp" "$file"
done

if grep -R '@TYPELESS_VERSION@' "$output_dir" >/dev/null 2>&1; then
  echo "The website build still contains an unresolved version placeholder." >&2
  exit 1
fi

if ! command -v zensical >/dev/null 2>&1; then
  echo "The documentation builder is missing: install requirements-docs.txt first." >&2
  exit 1
fi

(
  cd "$project_dir"
  zensical build --clean --strict
)

test -f "$output_dir/docs/index.html"
echo "Built typeless-ibus website version $version and documentation in $output_dir"
