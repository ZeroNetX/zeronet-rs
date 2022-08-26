# Changelog

## [0.1.2] - 2022-08-26

- Initial Plugin Implementaion to extend via Engine Plugins.
- Added `cryptKeyPair`, `cryptSign`, `cryptVerify` cmds to generate keys and sign and verify data.
- Added `pluginSign` and `pluginVerify` cmds to sign and verify plugins.
- Several Internal changes and bug fixes, See [diff](https://github.com/canewsin/zeronet-rs/compare/v0.1.1...v0.1.2) for full commit changes.

## [0.1.1] - 2022-08-12

- Save Site Storage to  sites.json when Site is Downloaded
- Set Site Serving to true in Storage by def when Site is Downloaded
- Set Def theme when user is created
- remove "/" from Def ZeroNet homepage addr in env def homepage
- Moved Site Files from ui to assets
- Provide Default peers.txt(onion peers) and trackers.txt(onion trackers, currently empty) via assets
- Use New Default peers and tracker for commuincation
- This release depends on tor, user needs to deploy tor on his Operating System before running ZeroNet for Communication to work.

## [0.1.0-patch] - 2022-08-11

- Rename COPYING to LICENSE.md
- Most of these are Github Action Changes for Binary Releases
- Include CHANGELOG.md, COPYING, README.md, ui directory with each release
- Name Build Steps 
- Regular Naming for Github Action Release Binaries
- Tag Custom OS tags for Releases
- Cancel Prev Builds if any 
- Cache build artfacts

## [0.1.0] - 2022-08-11

- Added CHANGELOG.md
- Automoted Public Releases via GitHub Actions.

## [0.0.1] - Before 2022-08-11

- Changelog is not maintained for these commits, but commit history has some what proper decription of changes.