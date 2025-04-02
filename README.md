# dweb
**dweb** is both an app for the Autonomi peer-to-peer network and a Rust library for others wanting to build decentralised web, desktop and command line applications.

If you have Rust installed you can take a look at the early dweb right now:
```
cargo install --locked dweb-cli
dweb open awesome
```

This is an active project with lots of ideas and potential, but already supports publishing and viewing decentralised websites without the need to setup hosting or domains, where every version of every website will be available for the lifetime of the Autonomi network.

**dweb's** capabilities and roadmap are described in the features section of the **dweb-cli** [README.md](https://github.com/happybeing/dweb/tree/main/dweb-cli/README.md#Features).

## Command Line Usage
To get the **dweb** command line app and view a website live on Autonomi:
```
cargo install --locked dweb-cli
dweb open awesome
```
After installing dweb this will open your web browser to show a website stored on Autonomi which contains links to other websites also on Autonomi. If you don't yet have Rust see [Get Rust](#get-rust).

In time, to avoid the need to install Rust, downloads will be made available for Windows, Mac OS and Linux.

## Library Usage
Browse the features of dweb on [docs.rs](https://docs.rs/dweb/latest/dweb/) and add dweb to your Rust project as normal with:

```
cargo add dweb
```
### Status
The dweb library is a work in progress so expect breaking changes expecially in newly added features. The web publishing format and command line interface are more stable but breaking changes are still possible.

## Contributing
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise.

## LICENSE

Everything is licensed under AGPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)
