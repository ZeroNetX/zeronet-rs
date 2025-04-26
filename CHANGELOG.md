# Changelog

## [0.3.1] - 2025-04-27
- Add details about rust toolchain(compile requirements) in `README.md`
- concurrent request for `peer_exchange` and more methods like `Site::find_peers`
- Add `Peer::connect_async` + `ZeroConnection::from_address_async` in `zeronet_protocol` crate
- adopt new nightly api changes for `extract_if` & unsafe block for `std::env::set_var`
- Several Internal changes and bug fixes, See [diff](https://github.com/ZeroNetX/zeronet-rs/compare/v0.2.0...v0.3.1) for full commit changes.

## [0.2.0] - 2024-01-23
- impl handle_site_bad_files
- impl handle_cert_(set/list)
- handle /Websocket route with serve_websocket
- handle_file_(get\need) improvements
- improve serve_websocket security
- impl handle_cert_select fn
- DBQueryRequest & SitesController::db_query > change method/struct sig to accept params
- SitesController::parse_query and tests
- Add a info log when UiServer starts
- UserSiteData handler 
- Fix Sites sub dir serving
- Several Internal changes and bug fixes, See [diff](https://github.com/ZeroNetX/zeronet-rs/compare/v0.1.3...v0.2.0) for full commit changes. 

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