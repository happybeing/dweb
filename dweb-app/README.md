# AutonomiDweb App

The AutonomiDweb App gives access to the dweb on Autonomi using a standard web browser. When running you can browse the dweb, and use dynamic web apps that upload files and save app data securely and privately on the Autonomi network.

This is somewhat like IPFS and similar projects but on Autonomi data is secured forever. So websites and data will always be available without the need for pinning, seeding or ongoing payments, or the need for users of developers to maintain or pay for servers.

The AutonomiDweb is always there, providing secure access to your data which is always encrypted and can't be accessed, deleted or blocked by anyone else.

### TODO:
- [x] support browsing and use of dynamic websites
- [ ] auto start on login
- [ ] system tray support
- [ ] visit websites on Autonomi
- [ ] add screenshot under "Browse the..."

## Browse the Autonomi dweb

1. Visit the [releases page](https://github.com/happybeing/dweb/releases) and click on the link for your computer (Windows, Mac or Linux)

2. Save the executable (`dweb-app.exe` or `dweb-app`) on your computer.

3. Run the app as normal and click the "Browse" button to begin browsing websites on Autonomi. This will open your browser, and may show an error at first, but once it has connected to Autonomi will load a website showing links to some early websites and apps on Autonomi.

## Web Publishers and Web Developers
If you wish to publish a website or a dynamic web app on Autonomi you will also need to download the dweb command line interface (dweb CLI).

Using the `dweb-cli` anyone can publish websites or dynamic web apps to Autonomi, accessible to anyone forever. In fact, using dweb ensures that every version will remain accessible, so there's no more link rot or expired domains, and no more lost data. This also puts a stop to ransomware because all your data is versioned when uploaded with dweb. The CLI also includes other features useful to developers, such as inspecting data structures on the live network.

Publishing websites or dynamic web apps using dweb is simple. Any static HTML website can be published and will be accessible in a standard browser just by the user running this `dweb-app`.

Web apps are able to access the Autonomi API via the REST APIs of the AutonomiDweb App.

## App Developers
Native apps can integrate the dweb server directly using the `dweb-server` crate, avoiding the need for a user to run a separate server using the `dweb-app` or `dweb-cli`. Those crates are useful examples of how to do this.

## dweb Autonomi REST API

For more about the REST API, and using `dweb-cli` to publish and view Autonomi websites and apps see: https://codeberg.org/happybeing/dweb/src/branch/main/dweb-cli#contents

# Development
This app was created using Tauri v2 and selecting the SvelteKit front end with Javascript.

For development, install Tauri v2 and its pre-requisites for your system (see https://v2.tauri.app)

```
git clone https://codeberg.org/happybeing/dweb
cd dweb/dweb-app
cargo tauri dev
```

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
