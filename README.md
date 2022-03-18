# ZeroNetX
Rust Implementation of ZeroNet Protocol

Currently this is just a non active cmd line tool. You can use available commands to execute things.

## Features:
 - Site Download => Download Site from internet peers
 - Find Peers via Trackers => Discover Peers using torrent tracker network
 - Site Verify => verify content file hashes with files
 - PeerExchange => Get more peers from connected peers
 - DbRebuild => Build db from data files using dbschema.json
 
## Available Commands:
 - siteDownload
 - siteFindPeers
 - dbRebuild
 - sitePeerExchange
 - siteFetchChanges
 - siteVerify

pass -s "Your Site Address" for above commands

## Download :
Latest Packages Available on [Github Releases](https://github.com/canewsin/ZeroNetX/releases/latest).

## Usage :
### Window :
> zeronet.exe siteDownload -s "1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d"
### Linux/Mac :
> ./zeronet siteDownload -s "1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d"

You may need to give exec permission on Linux/Mac OS
> chmod +x zeronet

and rerun the cmd