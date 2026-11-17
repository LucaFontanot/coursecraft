#!/bin/sh
# Prints the CHANGELOG.md section for one version, without its heading,
# for the release description.
# usage: release-notes.sh <version>   (for example 0.2.0 or v0.2.0)
set -eu
version=${1#v}
notes=$(awk -v v="$version" '
  $0 == "## " v || index($0, "## " v " ") == 1 { found = 1; next }
  found && /^## / { exit }
  found { print }
' "$(dirname "$0")/../CHANGELOG.md" | sed -e '/./,$!d')
if [ -z "$notes" ]; then
  echo "CHANGELOG.md has no section for $version" >&2
  exit 1
fi
printf '%s\n' "$notes"
