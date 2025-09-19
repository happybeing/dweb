# dweb
**dweb** is a project for publishing and browsing of websites and dynamic web apps in a standard browser on the Autonomi peer-to-peer network. It includes:
- [dweb-app](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-app): an app for browsing the Autonomi dweb
- [dweb-cli](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-cli): a command line app for publishing websites and many utility features
- [dweb-server](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-server): a Rust lib so your app can open Autonomi dweb apps and websites in a standard browser
- [dweb-lib](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-lib): a Rust lib containing core dweb features

In addition, [dweb-server-tauri-app](https://codeberg.org/happybeing/dweb-server-tauri-app) shows how to embed the dweb-server in a Tauri app:

**dweb's** capabilities and roadmap are described in the features section of the **dweb-cli** [README.md](https://github.com/happybeing/dweb/tree/main/dweb-cli/README.md#Features).

This is an active project with lots of ideas and potential, but already supports publishing and viewing decentralised websites without the need to setup hosting or domains, where every version of every website will be available for the lifetime of the Autonomi network.

You can 'Get dweb' either by installing Rust or downloading `dweb` for Windows, Mac OS and Linux.

Bug reports and a note for developers: feature requests and issues should be opened on Codeberg [here](https://codeberg.org/happybeing/dweb/issues). The github repository is a mirror, only used to build releases.

## Get dweb
You can download the latest binaries Windows, MacOS and Linux from [here](https://github.com/happybeing/dweb/releases).

Or you can install using Rust:
```
cargo install dweb-cli
```
Notes:
- If you don't yet have Rust see [Get Rust](#get-rust)
- If `cargo install dweb-cli` fails, try `cargo install --locked dweb-cli`


## Command Line Usage
To browse websites on Autonomi, you can open a default website (awesome) like this:
```
dweb
```
Notes:
- When the browser opens it reports an error but the page will load after the server has fetched the default website (awesome) from Autonomi.

dweb has many subcommands which you can view using `dweb --help`. Here's one which let's you go directly to a particular website:
```
dweb open gameboy
```
You can provide the address of a website on Autonomi (as a long hexadecimal string) or for some sites, a name such as 'awesome' or 'friends'.

To see the list of website names available:
```
dweb list-names
```

### Server concurrency (workers)

The dweb HTTP server (Actix) runs with a configurable number of worker threads.

- Environment variable: `DWEB_WORKERS`
- Default: `12`
- Example:
  ```bash
  DWEB_WORKERS=24 dweb
  ```

Notes:
- More workers can increase throughput for I/O-bound workloads at the cost of CPU/RAM/file descriptors.
- A good starting point is near the number of logical CPU cores. Measure and adjust.

## Status and Documentation
The dweb library is a work in progress so expect breaking changes expecially in newly added features. The web publishing format and command line interface are more stable but breaking changes are still possible.

Each module contains its own README but the dweb-cli README (see [Contents](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-cli#contents)) provides the most comprehensive documentation at the present time, including a roadmap of possible features. Requests, feedback and bug reports are welcome - please open an issues on Codeberg [here](https://codeberg.org/happybeing/dweb/issues).

## Contributing
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise.

## LICENSE

Everything is licensed under AGPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)
