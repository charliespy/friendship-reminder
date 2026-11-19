# Changelog

All notable changes are written here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed
- Friends is now a Tauri app (Rust + a web UI) instead of a Swift app.
- Friends keeps its own list and sends its own notifications, instead of writing to Apple
  Reminders.

### Added
- Unit tests for the scheduling rules and end-to-end tests of every screen, run on every PR.
- Releases: pushing a `v*` tag builds the app and publishes it on GitHub Releases.
