# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Transparent session token refresh on expiry (API error code 119) with automatic re-authorization and retry
- Structured `Auth` and `Api` variants to `SynoError` to preserve Synology error codes

### Changed

- API errors now consistently preserve the Synology error code across all methods
- `authorize()` no longer requires `&mut self` (now takes `&self`)
- `is_authorized()` is now an `async` method
- `SynoError` is now marked `#[non_exhaustive]`

### Removed

- `SynoError::SessionExpired` variant (session expiry is handled transparently via auto-retry)

## [0.4.0] - 2026-02-01

### Added

- Ratio calculation utility function (PR #3 by @shantheone)

## [0.3.0] - 2026-01-31

### Changed

- Request all additional fields (transfer, tracker, peer, file, detail) in task details (#1)

## [0.2.0] - 2025-05-25

### Added

- `is_authorized()` method

### Changed

- Renamed `host` parameter to `url` across the API

## [0.1.0] - 2025-04-13

### Added

- Initial release
- Authentication with Synology API (session-based)
- List all download tasks with detailed information
- Get specific task details
- Create downloads from URLs, HTTPS links, and magnet links
- Create downloads from torrent files (multipart upload)
- Pause, resume, complete, and delete tasks
- Clear completed downloads
- Human-readable file sizes, progress, and ETA utilities
- Builder pattern for client configuration
