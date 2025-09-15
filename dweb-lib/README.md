# dweb
**dweb** is a Rust library which supports publishing and viewing websites on the decentralised web of the [Autonomi](https://autonomi.com) peer-to-peer network. With it you build apps which publish and view of decentralised websites and use helpers which simplify some areas of the Autonomi API.

- Capabilities and roadmap: see the features section of the **dweb-cli** [README.md](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-cli#current-features-and-future-plans).

- dweb API: see [docs.rs](https://docs.rs/dweb/latest/dweb/).

- Add dweb to your Rust project:

  ```
    cargo add dweb
  ```

## dweb REST services

If you wish to include the dweb REST services in your app, see the module:
- [dweb-server](https://codeberg.org/happybeing/dweb/src/branch/main/dweb-server): a Rust crate for embedding dweb web browsing in a command line app or a Tauri app

## Status
The dweb library is a work in progress so expect breaking changes expecially in newly added features. The web publishing format and command line interface are more stable but breaking changes are still possible.

## Contributions
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise. Any contributions are accepted on the condition they conform to that license and the following conditions:

- that by submitting a contribution you are confirming that you are the sole author, understand all the submitted code in depth, and that no AI or other code generation tool that has ingested copyright material was used in generating content in your contribution.

Thanks for understanding that I don't want to accept material of unknown provenance nor spend time reviewing code that the contributor doesn't understand completely.

## LICENSE

Everything is licensed under AGPL3.0 unless otherwise stated.

See also [../LICENSE](../LICENSE)
