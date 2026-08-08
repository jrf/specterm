#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/.." && pwd)"

version="${1:-}"
output_dir="${2:-$repo_dir/dist}"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
    echo "usage: $0 <version> [output-directory]" >&2
    echo "version must look like 1.2.3 or 1.2.3-beta.1" >&2
    exit 2
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "specterm releases must be packaged on macOS." >&2
    exit 1
fi

package_version="$(awk -F '"' '/^version = "/ { print $2; exit }' "$repo_dir/Cargo.toml")"
if [[ "$version" != "$package_version" ]]; then
    echo "Release version $version does not match Cargo.toml version $package_version." >&2
    exit 1
fi

architecture="$(uname -m)"
case "$architecture" in
    arm64 | x86_64) ;;
    *)
        echo "Unsupported release architecture: $architecture" >&2
        exit 1
        ;;
esac

code_sign_identity="${SPECTERM_CODE_SIGN_IDENTITY:--}"
notary_key_path="${NOTARY_KEY_PATH:-}"
notary_key_id="${NOTARY_KEY_ID:-}"
notary_issuer_id="${NOTARY_ISSUER_ID:-}"

notary_value_count=0
[[ -n "$notary_key_path" ]] && notary_value_count=$((notary_value_count + 1))
[[ -n "$notary_key_id" ]] && notary_value_count=$((notary_value_count + 1))
[[ -n "$notary_issuer_id" ]] && notary_value_count=$((notary_value_count + 1))
if [[ "$notary_value_count" -ne 0 && "$notary_value_count" -ne 3 ]]; then
    echo "NOTARY_KEY_PATH, NOTARY_KEY_ID, and NOTARY_ISSUER_ID must be provided together." >&2
    exit 1
fi
if [[ "$notary_value_count" -eq 3 && "$code_sign_identity" == "-" ]]; then
    echo "Notarization requires Developer ID-signed binaries." >&2
    exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/specterm-release.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

cargo_target_dir="$work_dir/cargo-target"
swift_scratch_dir="$work_dir/swift-build"
staging_dir="$work_dir/dmg"
dmg_name="specterm-${version}-macOS-${architecture}.dmg"
dmg_path="$output_dir/$dmg_name"

CARGO_TARGET_DIR="$cargo_target_dir" cargo build \
    --manifest-path "$repo_dir/Cargo.toml" \
    --release \
    --locked
swift build \
    --package-path "$repo_dir/tap" \
    --scratch-path "$swift_scratch_dir" \
    -c release

mkdir -p "$staging_dir" "$output_dir"
install -m 755 "$cargo_target_dir/release/specterm" "$staging_dir/specterm"
install -m 755 "$swift_scratch_dir/release/specterm-tap" "$staging_dir/specterm-tap"
install -m 644 "$repo_dir/README.md" "$staging_dir/README.md"

if [[ "$code_sign_identity" == "-" ]]; then
    codesign --force --options runtime --sign - "$staging_dir/specterm"
    codesign --force --options runtime --sign - "$staging_dir/specterm-tap"
else
    codesign --force --options runtime --timestamp \
        --sign "$code_sign_identity" "$staging_dir/specterm"
    codesign --force --options runtime --timestamp \
        --sign "$code_sign_identity" "$staging_dir/specterm-tap"
fi

codesign --verify --strict --verbose=2 "$staging_dir/specterm"
codesign --verify --strict --verbose=2 "$staging_dir/specterm-tap"

rm -f "$dmg_path" "$dmg_path.sha256"
hdiutil create \
    -quiet \
    -volname "specterm $version" \
    -srcfolder "$staging_dir" \
    -format UDZO \
    -ov \
    "$dmg_path"

if [[ "$code_sign_identity" != "-" ]]; then
    codesign --force --sign "$code_sign_identity" --timestamp "$dmg_path"
fi

if [[ "$notary_value_count" -eq 3 ]]; then
    xcrun notarytool submit "$dmg_path" \
        --key "$notary_key_path" \
        --key-id "$notary_key_id" \
        --issuer "$notary_issuer_id" \
        --wait
    xcrun stapler staple "$dmg_path"
    xcrun stapler validate "$dmg_path"
    spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg_path"
fi

(
    cd "$output_dir"
    shasum -a 256 "$dmg_name" > "$dmg_name.sha256"
)

echo "Created $dmg_path"
echo "Checksum: $dmg_path.sha256"
