# dweb CLI
**dweb** is a command line program which enables you to publish and view websites on the decentralised web aboard the [Autonomi](https://autonomi.com) peer-to-peer network.

**WIP NOT YET FUNCTIONAL:** This is a work-in-progress but is preceded by a working demo called [awe](https://github.com/happybeing/awe) which already allows websites to be published and viewed on Autonomi.

Unlike `awe`, **dweb** will work with standard browsers to view decentralised websites directly from the Autonomi peer-to-peer network. Whereas the `awe` demo includes a primitive web viewer instead.

## Features
**dweb** will support a superset of features demonstrated in `awe`, itemised in the following roadmap or 'to-do' list.

### Roadmap Status
- [x] **dweb publish-new | publish-update** - commands to publish and update permanent websites on a decentralised web, which means no 'link rot' (links that stop working because a domain expires etc). Permanence is a unique feature of data stored on Autonomi. By default websites are accessible by anyone (public data).

- [x] **dweb serve** - run a local server for viewing dweb websites in a standard web browser. Since websites are versioned, you can view every version of every website published using **dweb**.

- [ ] **api-rclone** - a RESTful HTTP API for an [rclone](https://github.com/rclone/rclone/) backend for Autonomi to support backup, mounting of decentralised storage, sync and copy between devices and other storage backends (e.g. cloud storage).

- [ ] **dweb upload |download | share | sync** - commands to upload and download data to/from your permanent decentralised storage on Autonomi. **dweb upload** stores data privately, although you can **dweb share** to override this and share files or directories with others, or with everyone. As with websites, uploaded data is versioned as well as permanent, so you will always be able to access every version of every file you have ever uploaded.

- [ ] **dweb service** - install, start, stop and remove one or more **dweb** APIs including the website server.
- [ ] **files-browser** - a built-in web app for managing your files stored on Autonomi.
- [ ] **api-solid** - a RESTful HTTP API for a [Solid](https://solidproject.org/about) 'Pod' using Autonomi to provide decentralised personal data storage.
- [ ] **api-webdav** - [tentative] a RESTful HTTP API giving access to Autonomi storage over the WebDAV protocol. This allow any app which supports WebDAV to access Autonomi decentralised storage. It is tentative because I think it might be a good first step towards creating the rclone backend API, rather than a priority itself.
- [ ] **autonomi-api** - [tentative] a RESTful HTTP version of part or all of the Autonomi API. It is tentative because Autonomi already support WASM for browser apps which may make this unnecessary.
- [x] **dweb inspect-history** - a command for interrogating Autonomi's versioned mutable storage for websites and files.
- [ ] **dweb inspect-files** - a command for for listing data about directories and files stored on Autonomi.

That's a long list for a one-person project so each area is available for others to contribute to, so if a feature is not implemented yet and you want it faster you might be able to make that happen! See 'Contributing' below.

Features already available in `awe` will arrive quickly once the infrastructure is in place and the relevant functionality has been incorporated in **dweb-lib**. That includes website publishing/viewing and file upload/download which are nearly or fully complete already (in `awe`)

## Contributing
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise.

## LICENSE

Everything is licensed under AGPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)
