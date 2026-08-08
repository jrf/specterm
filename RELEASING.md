# Releasing specterm

specterm is distributed as two Developer ID-signed command-line binaries:
`specterm` and its ScreenCaptureKit helper, `specterm-tap`. Tagging a release
builds notarized DMGs for Apple silicon and Intel Macs.

## Required GitHub Actions secrets

- `DEVELOPER_ID_APPLICATION_CERTIFICATE_BASE64`: base64-encoded `.p12`
  containing the Developer ID Application certificate and private key.
- `DEVELOPER_ID_APPLICATION_CERTIFICATE_PASSWORD`: password used when the
  `.p12` was exported.
- `DEVELOPER_ID_APPLICATION_IDENTITY`: full signing identity, for example
  `Developer ID Application: Example Name (TEAMID)`.
- `APP_STORE_CONNECT_API_KEY_ID`: App Store Connect API key identifier.
- `APP_STORE_CONNECT_API_ISSUER_ID`: App Store Connect issuer identifier.
- `APP_STORE_CONNECT_API_PRIVATE_KEY`: contents of the API key `.p8` file.

Never store any of these values in the repository.

## Local packaging check

The version must match `Cargo.toml`. Build an ad-hoc-signed DMG with:

```fish
just package 0.1.0
```

This validates both release builds, code-signature integrity, DMG creation, and
checksum generation. It does not produce a distributable release.

## Publishing

After updating `Cargo.toml` and `Cargo.lock`, put the release commit on the
default branch, then create and push a signed semantic version tag:

```fish
git tag -s v0.1.0 -m "specterm 0.1.0"
git push origin v0.1.0
```

The release workflow signs and notarizes both architecture-specific DMGs, then
publishes them and their SHA-256 checksums to one GitHub release.
