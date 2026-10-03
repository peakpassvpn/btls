#!/usr/bin/env bash
# The BoringSSL this fork publishes for the btls-sys a Cargo.lock pins:
# finds the pinned commit, downloads boringssl-<target>.tar.gz from the
# release bssl-<commit>, checks it against the release's SHA256SUMS and its
# build attestation (gh attestation verify), unpacks it, and prints the
# environment that has btls-sys link it instead of compiling BoringSSL.
#
#   eval "$(prebuilt-boringssl.sh path/to/Cargo.lock x86_64-pc-windows-msvc)"
#
# Needs gh (signed in), curl and tar. The files go into
# ${BTLS_BORINGSSL_CACHE:-~/.cache/btls-boringssl}/<commit>/<target>, kept
# for the next build. The variables are the target's own
# (BORING_BSSL_PATH_<target>), so a build that also compiles for its host
# is not handed the target's library for it.

set -euo pipefail

lock=${1:?usage: $0 <Cargo.lock> <target>}
target=${2:?usage: $0 <Cargo.lock> <target>}
repo=peakpassvpn/btls

commit=$(awk '
	/^name = "btls-sys"$/ { found = 1; next }
	found && /^source = / {
		if (match($0, /#[0-9a-f]+"$/)) print substr($0, RSTART + 1, RLENGTH - 2)
		exit
	}
	/^\[\[package\]\]/ { found = 0 }
' "$lock")
if [ -z "$commit" ]; then
	echo "$lock pins no btls-sys from git" >&2
	exit 1
fi
case $commit in
*[!0-9a-f]* | "") echo "not a commit: $commit" >&2 && exit 1 ;;
esac
[ ${#commit} -eq 40 ] || {
	echo "not a full commit: $commit" >&2
	exit 1
}

dir=${BTLS_BORINGSSL_CACHE:-$HOME/.cache/btls-boringssl}/$commit/$target
if [ ! -f "$dir/BUILDINFO" ]; then
	file=boringssl-$target.tar.gz
	work=$(mktemp -d)
	trap 'rm -rf "$work"' EXIT
	base=https://github.com/$repo/releases/download/bssl-$commit
	curl -fsSL --retry 3 -o "$work/$file" "$base/$file" >&2
	curl -fsSL --retry 3 -o "$work/SHA256SUMS" "$base/SHA256SUMS" >&2
	want=$(awk -v f="$file" '$2 == f || $2 == "*" f { print $1 }' "$work/SHA256SUMS")
	if command -v sha256sum >/dev/null; then
		got=$(sha256sum "$work/$file" | cut -d' ' -f1)
	else
		got=$(shasum -a 256 "$work/$file" | cut -d' ' -f1)
	fi
	if [ -z "$want" ] || [ "$want" != "$got" ]; then
		echo "$file: sha256 $got, SHA256SUMS says ${want:-nothing}" >&2
		exit 1
	fi
	gh attestation verify "$work/$file" -R "$repo" >&2
	mkdir -p "$dir.tmp"
	tar -xzf "$work/$file" -C "$dir.tmp"
	rm -rf "$dir"
	mv "$dir.tmp" "$dir"
fi

t=${target//-/_}
echo "export BORING_BSSL_PATH_$t='$dir'"
echo "export BORING_BSSL_INCLUDE_PATH_$t='$dir/include'"
echo "export BORING_BSSL_ASSUME_PATCHED_$t=1"
