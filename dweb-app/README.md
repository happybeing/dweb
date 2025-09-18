# dweb-app - local dweb server for dweb Autonomi (REST API)

This is a simple app that gives access to the dweb on Autonomi.

It provides a local server so you can use a standard browser to view websites and use dynamic apps that load directly from the Autonomi secure peer-to-peer network. This is like IPFS and similar projects but websites and data will always be available without the need for pinning, seeding or ongoing payments.

The Autonomi dweb is always there, providing secure access to your data which is always encrypted and can't be accessed, deleted or blocked by anyone else.

## Status
This app was created using Tauri v2 and and SvelteKit and is in development.

### TODO:
- [ ] installation options and pre-requisites
- [ ] visit websites on Autonomi
- [ ] add screenshot
- [ ] link to other READMEs

## Try the Autonomi dweb

### Install dweb-app
TODO

## Web Publishers
Using the `dweb-cli` anyone can publish websites or dynamic web apps to Autonomi, accessible to anyone forever.

In fact, using dweb ensures that every version will remain accessible, so there's no more link rot or expired domains, and no more lost data. This also puts a stop to ransomware because all your data is versioned when uploaded with dweb.

## Web Developers
Publishing websites or dynamic web apps using dweb is simple. Any static HTML website can be published and will be accessible in a standard browser just by the user running this `dweb-app`.

For web apps, you will be able to access the Autonomi API via the REST APIs provided by this `dweb-app`.

## App Developers
Apps can integrate the dweb server directly using the `dweb-server` crate, avoiding the need for a user to run a separate server using this `dweb-app` or `dweb-cli`. Those crates are useful examples of how to do this.

## dweb Autonomi REST API

For more about the REST API, and using `dweb-cli` to publish and view Autonomi websites and apps see: https://codeberg.org/happybeing/dweb/src/branch/main/dweb-cli#contents

# Development

Install Tauri v2 and its pre-requisites for your system (see https://v2.tauri.app)

```
git clone https://codeberg.org/happybeing/dweb-server-tauri-app
cd dweb-server-tauri-app
cargo tauri dev
```
You can use this app to understand how to integrate a dweb REST server within your binary, or as a template for your own app.

This app was created using Tauri v2 and selecting Rust GUI with Vanilla HTML, CSS and Javascript.

## Recommended IDE Setup
 HTML,
- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Contributions
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise. Any contributions are accepted on the condition they conform to that license and the following conditions:

- that by submitting a contribution you are confirming that you are the sole author, understand all the submitted code in depth, and that no AI or other code generation tool that has ingested copyright material was used in generating content in your contribution.

Thanks for understanding that I don't want to accept material of unknown provenance nor spend time reviewing code that the contributor doesn't understand completely.

## Contributions
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise. Any contributions are accepted on the condition they conform to that license and the following conditions:

- that by submitting a contribution you are confirming that you are the sole author, understand all the submitted code in depth, and that no AI or other code generation tool that has ingested copyright material was used in generating content in your contribution.

Thanks for understanding that I don't want to accept material of unknown provenance nor spend time reviewing code that the contributor doesn't understand completely.

## LICENSE

Everything is licensed under AGPL3.0 unless otherwise stated.

See also [../LICENSE](../LICENSE)
