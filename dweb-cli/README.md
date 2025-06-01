# dweb Command Line App
**dweb** is for use with the Autonomi peer-to-peer network. Features are being added all the time, but already include:

- viewing the decentralised web in any standard browser, directly on Autonomi over end-to-end encrypted connections

- publishing of decentralised websites created using standard web tooling (e.g. Publii, Svelte, static site generators or plain HTML/CSS)

- a local web server/service for websites on Autonomi, and will provide built in web apps for things like file management.

- RESTful and Rust APIs for dynamic websites and desktop apps

- backup and sync using [rclone](https://github.com/rclone/rclone/) (is planned)

Web apps are easy and quick to build:
- **Publii** - build blogs and websites using this third party desktop app
- **Names** - list, create and delete memorable names for Autonomi websites
- **Fileman** - view versioned files on Autonomi by address or name

You can use any **static site generator** that does what you need.

### Status
**IMPORTANT:** **dweb** and **Autonomi** are at an early stage so regard this as experimental (pre-alpha). Everything here works on a local test network and can be used to try things out or begin building web apps for Autonomi.

Note: instructions below have been written for the public network, so be aware that most things only work on a test network for now. Some have had success publishing on the Autonomi public network, but it will be much easier to begin with a local testnet. To start a local testnet see [Test your App](#test-your-app).

### Quickstart dweb Browsing
If you have Rust installed you can view websites live on Autonomi in two steps:
```
cargo install --locked dweb-cli
dweb open awesome
```
The above opens your browser and loads a website from Autonomi containing links to other sites you can view. Just a taste of things to come. More demo sites are welcome, and will be included to help people get started on the dweb.

## Contents
- [Browse the DWeb](#browse-the-dweb)
- [Publish a Website](#publish-a-website)
- [About the Decentralised Web (DWeb)](#about-the-decentralised-web-dweb)
- [Current Features and Future Plans](#current-features-and-future-plans)
- [Develop a Decentralised Web App](#develop-a-decentralised-web-app)
- [Contributing](#contributing)
- [LICENSE](#license)

## Browse the dweb

- [Get Rust](#get-rust)
- [Install dweb-cli](#install-dweb-cli)
- [Browse websites on Autonomi](#browse-websites-on-autonomi)
- [Advanced Browsing](#advanced-browsing)

### Get Rust

In time, downloads will be provided to avoid the need to install Rust, but until then:

- **MacOS and Linux:** use `rustup` as explained here: [Install Rust](https://www.rust-lang.org/tools/install)

- **Windows users:** visit [Install Rust](https://www.rust-lang.org/tools/install) and see "Other Installation Methods" link on that page. For most Windows users I suggest scrolling down to find the first `x86_64-pc-windows-msvc` link and click on that.

### Install dweb-cli
```
cargo install dweb-cli --locked
```
Note: when installing dweb on Ubuntu, you may encounter missing openssl libraries. Do a `sudo apt update` and then `sudo apt install librust-openssl-dev`, and finally restart the dweb install.

Once you have `dweb` installed you'll be able to find other websites and apps in the **awesome index** with `dweb open awesome`.

If you need help, you can list the dweb subcommands and options with:
```
dweb help
```

### Browse websites on Autonomi
To browse the dweb, first start the server. This only needs to be done once until you reboot or shut it down:
```
dweb serve
```
Until you know some websites on Autonomi you can start at the *dweb-awesome-links* website which contains links to websites built by the community. Type:
```
dweb open awesome
```

If you know the xor address of a website you can browse it like this:

```
dweb open 8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
```
The above is the long string is the address of the awesome website, so change part to the address of the site you wish to view.

To open a website and give it a name:
```
dweb open --as-name toast b89dbdad3297bde6539723b63f92a508bccf6ba6b0956b9f2aad6d139260d41c36256b3fa3a8394c9ec990d5e45e6c71
```
You can also just name sites yourself and then use those names with 'dweb open':
```
$ dweb name toast b2691ea46cd73dc07b1c5f74803b3b99cb83e6a308d026c00cb683d37cde619fe2c55778be67ea8d5c2d1e3b2a95bb83
$ dweb list-names
awesome                                  8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
toast                                    b89dbdad3297bde6539723b63f92a508bccf6ba6b0956b9f2aad6d139260d41c36256b3fa3a8394c9ec990d5e45e6c71
```
Then:
```
$ dweb open toast
```
Names are not persistant yet, so they will be forgotten when you restart the server.

### Advanced Browsing
There are some neat features of the dweb which you can access via a regular browser while viewing a dweb website. These include getting information about the website, choosing which version of a website you want to view, or opening another website.

These features involve you editing the URL in the address bar of your browser. This is a bit clunky, but at some point someone may create a plugin to simplify this (hint!)

IMPORTANT:
- when using these features be careful not to change the part of the URL up to and including the PORT, which is the number 44827 in the URL: `http://127.0.0.1:44827`

- every dweb website you view will use a different number so you mustn't change this part when editing.

- the URLs in the following examples will not work for you because the PORT will be different each time you open a site on your system.

**/dweb-info** will show information about the website you are viewing, such as how many versions there are and the address of the website on Autonomi (useful for sharing).

For example, if you are viewing a site and the address bar contains the following:

```
http://127.0.0.1:44827/more-ants.html
```

Change this, being careful not to change anything up to and including the PORT (in this example 44827):

```
http://127.0.0.1:44827/dweb-info
```
When you press the ENTER key this will display a page about the current website, something like:

```
/dweb-info for History
HistoryAddress: 8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
ArchiveAddress:
515e9480edbecc277cef03ac6d8748afe3cbad8d09efaf8d0e603fcd7f8b21c0
Current version: most recent

Max version from pointer: 5
Max version from graph: not checked
```

**/dweb-version** changes the version of the website you are viewing.

If you want to view a particular version, say version 3, change '/dweb-info' to '/dweb-version/3' and press ENTER. To view the latest version use: '/dweb-version/latest'.

**/dweb-open** opens a website at a given address. Add the address (or a name which is recognised by the local server).

For example, to open the most recent version use 'v' without a version number, or provide the number of the version you wish to ope.

Open version 2 by including 'v2':
```
http://127.0.0.1:44827/dweb-open/v2/9188ec4c126c2fdcaceaf4a50ab18e28446b992ef1c5061789ed7af7e844343e71786cb3f69c10d6e98d6e018235709d
```
Open the most recent version leaving out the version number and using 'v':
```
http://127.0.0.1:44827/dweb-open/v/9188ec4c126c2fdcaceaf4a50ab18e28446b992ef1c5061789ed7af7e844343e71786cb3f69c10d6e98d6e018235709d
```

**/dweb-open-as** is similar but allows you to specify a dweb name for use with the local server. So to give the site the dweb name 'testing' use:

```
http://127.0.0.1:44827/dweb-open-as/v/testing/9188ec4c126c2fdcaceaf4a50ab18e28446b992ef1c5061789ed7af7e844343e71786cb3f69c10d6e98d6e018235709d
```
After which you can open it with the name 'testing', both in the browser with or on the command line.

Browser address bar:

```
http://127.0.0.1:44827/dweb-open/v/testing/
```
Command line:
```
dweb open testing
```


## About the Decentralised Web (DWeb)
- [The Permanent Web](#the-permanent-web)
- [Publish a Website](#publish-a-website)
- [Linking to Websites on Autonomi](#linking-to-websites-on-autonomi)
- [Browse your Website on Autonomi](#browse-your-website-on-autonomi)
- [Set up a Wallet](#set-up-a-wallet)

A decentralised web means having everything we have now but with autonomy and freedoms baked in such as:
- always on access free from service shutdown or failure
- data secured against hacking and surveillance
- publishing free from censorship and targeting

Using dweb you can publish a website without learning about domain names or servers in a single command.

For now dweb supports static websites built using regular web tooling with no changes needed. Even WordPress like blogs can be published as demonstrated using Publii ([visit getpublii.com](https://getpublii.com/)). If you have the dweb server running, you can get a taste of what other people have made so far with the command `dweb open awesome`.

As features are added to the dweb API, increasingly dynamic sites will be supported so that website builders can create a rich web experience using a familiar style of 'RESTful' API, using all their favourite tools.

The most difficult part of this will be setting yourself up with the means to pay for the storage, but you can simplify this by running some Autonomi 'nodes' to earn the tokens needed to pay for storage.

### The Permanent Web
Autonomi is designed to secure public and private data for the lifetime of the network for a one-off storage fee.

So using dweb for publishing on Autonomi ensures that every version of a website can be accessed even after new versions are published. This is like having the Internet Archive built into the web, and can be used to eliminate the problem known as 'link rot' where links stop working when websites are taken down or domains expire.

### Publish a Website
Publishing your website is a one line command, and a similar command to update it later. Each dweb site has it's own history which ensures past and present versions available forever.

When you publish, you will have to pay the Autonomi decentralised network a one-off upload fee. In fact you pay some of the nodes which will store your data. The cost is good value since there are no recurring fees once your data has been uploaded.

So before you can publish anything you need to set up a wallet with tokens, see [Setup a Wallet](#set-up-a-wallet)

Publication is a transaction between you and a decentralised peer-to-peer network, so no gatekeepers or intermediaries are involved.

For example, to publish a new website that is in a subdirectory 'blog' you would type:

```
dweb pubish-new --files-root blog
```
The index file to that website will be at blog/index.html. After making changes, update it with:
```
dweb pubish-update --name blog --files-root blog
```
By default, dweb uses the name of the directory containing the files as a name for the website when you later want to update it. You can though choose a different name when you `pubish-new` using the `--name` option of the subcommand.

Note: the publish-new name is local to you and only used with the `publish-update` subcommand.

For example, I use Publii to create a blog which is located in a directory called 'the-internet-burned-my-toast-again', but the 'index.html' file is in a subdirectory called 'output'.

The command used to publish this for the first time was:
```
dweb pubish-new --name toast --files-root the-internet-burned-my-toast-again/output
```
Whenever I update it I can refer to it by the name 'toast':
```
dweb pubish-new --name toast --files-root the-internet-burned-my-toast-again/output
```
Although dweb attempts to upload the whole of your website content when you do an update, you will only need to pay to upload any files which have changed. This is because Autonomi uses content addressing, and you never have to pay for a file that has already been uploaded by you or anyone else.

### Linking to Websites on Autonomi

Links on Autonomi use the /dweb-open and /dweb-open-as features described earlier, except you must only include the part from /dweb-open onwards.

Don't include the `http://127.0.0.1:44827` part.

The part you want might look like this:
```
/dweb-open/v/8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
```

And in HTML:
```html
<a href='/dweb-open/v/8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c'>click me</a>
```

### Browse your Website on Autonomi
When you publish your website, dweb prints instructions for how to browse it and a link to share with others using dweb. So look at the terminal output and make a note of the key parts after you publish the first version.

For example, after publishing my blog I can open it from the command. But first I must have a dweb server running on my computer.

You only have to do this once after reboot:

`dweb serve`

As long as the server is running, in another terminal I can view my blog using:
```
dweb open 8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
```

The above command is printed to the terminal whenever you publish or update your website, so make a note of it when you want to view or share with others.

That's a bit cumbersome, so you can give any website a 'dweb name' like this:
```
dweb open --as-name toast 8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
```
Or if you just want to set the name without opening it:
```
dweb name toast b2691ea46cd73dc07b1c5f74803b3b99cb83e6a308d026c00cb683d37cde619fe2c55778be67ea8d5c2d1e3b2a95bb83
```

Once named, I can open the website just using the name:
```
dweb open toast
```

Oh, and there's a built-in dweb name which you can use as soon as the server is running. This opens the website of awesome dweb sites built by the community so far:
```
dweb open awesome
```

When you have set a few names you can list them with:
```
dweb list-names
```

Notes about dweb names:
- although the publish-update command recognised the name 'toast' this is not available for use with `dweb open` or other commands which can accept a name until you have registered it with the running dweb server.
- dweb names are not yet stored and so will be forgotten whenever you restart the dweb server.

### Set up a Wallet
You will need a wallet if you wish to upload data to Autonomi, or use a web application which stores data on the network.

You don't need a wallet just to browse websites or download files published by others. If you want to upload data, you can set-up your wallet as follows.

1. Check if you have a wallet using the `ant-cli` (install with `cargo install ant-cli --locked` if you don't have that yet):
   ```
   ant wallet balance
   ```
2. If you don't have a wallet yet, create it with `ant wallet create` or import an existing wallet for which you already have the private key, with `ant wallet import`

3. If you don't have funds, head over to the [Autonomi Community Faucet](https://forum.autonomi.community/t/community-faucet-live/41299?u=ambled)

If you need help with this, see the help and support sections of `autonomi.com`:
- User focussed documentation ([docs.autonomi.com](https://docs.autonomi.com))
- Autonomi support ([Discord](https://discord.gg/autonomi))
- Community forum ([Discourse](https://forum.autonomi.community/))

Payment is handled automatically, and you can check the cost beforehand using `dweb cost` as follows:
```
dweb cost --files-root blog
```
At the time of writing the cost is not accurately reported by the Autonomi network, but is usually very cheap compared to cloud storage. Especially as your data will be stored for the lifetime of the Autonomi network at no extra cost.

#### SECRET_KEY variable
By default dweb will use the wallet you created or imported with the `ant` command (see above). If you have set a password for this, dweb will ask you for that so it can unlock the wallet when you run `dweb serve`. For some users and developers you can use a different wallet by setting the `SECRET_KEY` environment variable to the value of a wallet secret key. When `SECRET_KEY` is set, dweb will use this instead.

On Linux and MacOS you can either put `export SECRET_KEY=<PRIVATE-KEY>` in your `.bashrc` and then *open a new terminal*, or pass it when you start the dweb server with:
   ```
   export SECRET_KEY=<PRIVATE-KEY> dweb serve
   ```

## Current Features and Future Plans
- [Current Features](#current-features)
    - [Command Line](#command-line)
    - [Web API](#web-api)
    - [Rust API](#rust-api)
- [Future Features Roadmap](#future-features-roadmap)

The design of dweb creates a lot of possibilities. One is to to expand the **RESTful access to Autonomi APIs** to make it easy to create powerful web apps served and storing their data on its secure, decentralised replacement for cloud services.

Another ambition is to provide backup applications via an **rclone** compatible backend, as an API in the dweb server.

Others include adding support for services like ActicityPub and Solid Pods.

For more about future possibilities, see  [Roadmap](https://github.com/happybeing/dweb/blob/main/dweb-cli/README.md#future-features-roadmap)


### Current Features

#### Command Line

- **dweb publish-new** | **publish-update** - commands to publish and update directories or websites on a decentralised web. Directories are versioned and stored permanently. So all versions of the files or website will always be available, no expiring domains or 'link rot' (links that stop working because a domain expires etc). Permanence is a unique feature of data stored on Autonomi. By default websites are accessible to anyone (public data).

- **dweb serve** - run a local server for viewing dweb websites in a standard web browser. Since websites are versioned, you can view every version of every website published using **dweb**.

- **dweb open awesome** - loads an 'awesome list' website, and serves as a demonstration. It links to websites created by dweb users who send them to be included, and shows how to use the dweb API to register a DWEB-NAME for a website stored on Autonomi. This forms part of the URL displayed in the browser address bar and will work until the server is shut down. Later these names and the sites they point to will be made persistant using storage on Autonomi.

- **dweb name** | **dweb list-names** - memorable names for websites that will be understood by your local server.

- **dweb inspect-history** - a command for interrogating Autonomi's versioned mutable storage for websites and files.
- **dweb inspect-files** - list directories and files stored on Autonomi.
- **dweb inspect-pointer** - show the state of an Autonomi Pointer, a mutable data type.
- **dweb inspect-graphentry** - interrogate a GraphEntry type stored on Autonomi

#### Web API

The dweb web API allows a website or desktop application to access dweb and Autonomi APIs over a RESTful interface. This gives access to both the Autonomi APIs and the dweb-lib APIs (which include extensions such as for versioned data using a content History).

I welcome requests for specific features and general design of the API.

dweb uses OpenAPI to document all the APIs, and these can be viewed by starting a dweb server and opening the Swagger UI:
```
dweb serve
```
Then in another terminal:
```
dweb openapi-docs
```
![Screenshot of Swagger UI](./misc/screenshot-swagger-ui.png?raw=true "Screenshot of Swagger UI")

A sample of each kind of API are listed below but the Swagger UI provides a complete reference and live playground.

Example APIs designed for manual input in the browser address bar:
- **/dweb-open** - open a website or directory by version (optional), address or name
- **/dweb-open-as** - open a website or directory by version (optional) or address, and register a dweb name with the server
- **/dweb-version** - select the most recent or a specified version of the displayed website
- **/dweb-info** - show information about the displayed website

Note: /dweb-open and /dweb-open-as are also used inside a website to link to other websites on Autonomi.

Example APIs intended for access by apps (Autonomi RESTful API):
- **/ant-0/chunk**           - individual chunks (up to 4MB)
- **/ant-0/data**           - arbitrary data (unlimited size)
- **/ant-0/archive-public** - metadata for stored (see also /archive-private)
- **/ant-0/scratchpad-public**    - re-writeable storage (see also /scratchpad-private)

Example APIs intended for access by apps (dweb extensions):
- **/dweb-0/form-upload-file-list** - multi-part upload of one or more files
- **/dweb-0/directory-load**    - given an address or name, returns a directory tree (Archive) in JSON format
- **/dweb-0/name-register**     - register a dweb name for an address
- **/dweb-0/name-list**         - get a list of dweb names registered with the local server

The above show a small sample of the APIs implemented so far. To see the full APIs visit the OpenAPI docs as explained above.

#### Rust API
dweb APIs are also accessible from Rust in dweb-lib. This includes selected HTTP APIs making it easier to access features without handling HTTP requests and responses directly.

The Rust APIs are documented at [docs.rs](https://docs.rs/dweb/latest/dweb/).

### Future Features Roadmap
I have many other ideas and may be working on one of those rather than the following, so if there's something you'd be interested in using or working on let me know.

The following are things I would like to support, in no particular order. This is a lot for one persons so if you wish to help please let me know. I have notes on most that I can share and will help where I can.

If you have **web front-end skills** there are plenty of things to improve or write from scratch here, which will make my part much easier and speed everything in this list up.

- [ ] **api-rclone** - a RESTful HTTP API for an [rclone](https://github.com/rclone/rclone/) backend for Autonomi to support backup, mounting of decentralised storage, sync and copy between devices and other storage backends (e.g. cloud storage).

- [ ] **dweb upload |download | share | sync** - commands to upload and download data to/from your permanent decentralised storage on Autonomi. **dweb upload** stores data privately, although you can **dweb share** to override this and share files or directories with others, or with everyone. As with websites, uploaded data is versioned as well as permanent, so you will always be able to access every version of every file you have ever uploaded.

- [ ] **dweb service** - install, start, stop and remove one or more **dweb** APIs including the website server.
- [ ] **files-browser** - a built-in web app for managing your files stored on Autonomi.
- [ ] **api-solid** - a RESTful HTTP API for a [Solid](https://solidproject.org/about) 'Pod' using Autonomi to provide decentralised personal data storage.
- [ ] **api-webdav** - [tentative] a RESTful HTTP API giving access to Autonomi storage over the WebDAV protocol. This allow any app which supports WebDAV to access Autonomi decentralised storage. It is tentative because I think it might be a good first step towards creating the rclone backend API, rather than a priority itself.
- [ ] **autonomi-api** - [tentative] a RESTful HTTP version of part or all of the Autonomi API. It is tentative because Autonomi already support WASM for browser apps which may make this unnecessary.

That's a long list for a one-person project so each area is available for others to contribute to, so if a feature is not implemented yet and you want it faster you might be able to make that happen! See 'Contributing' below.

## Develop a Decentralised Web App
- [Setting Up](#setting-up)
- [Create your own dweb App](#create-your-own-dweb-app)
- [Develop your App](#develop-your-app)
- [Test your App](#test-your-app)
- [Build your App](#build-your-app)

The following instructions assume using the Svelte framework, but any web tooling will do so long as it allows you to create a static website.

### Example dweb Apps
Several examples are available to show how to build a dweb app:
- **ScratchChat** - chat anonymously over Autonomi. A single file of hand-crafted HTML. [github](https://github.com/happybeing/dweb-app-scratchchat)
- **Names** - list, create and delete memorable names for Autonomi websites. A Svelte app, see [github](https://github.com/happybeing/dweb-app-names)
- **Fileman** - viewing versioned files on Autonomi by address or name. A Svelte app, see [github](https://github.com/happybeing/dweb-app-fileman)

You can also use **static site generators** if they do what you need rather than building an app from scratch. So you can use Publii for example, to create a WordPress like blog and use dweb to upload it to Autonomi.

The instructions below refer to using Fileman to show how things work.

### Setting Up

Install the following:
- get Rust  for installing command line apps. See [Get Rust](https://www.rust-lang.org/tools/install)
- get dweb:
```
   cargo install --locked dweb-cli
   dweb --version
```
- get ant (the Autonomi command line interface):
```
   cargo install --locked autonomi-cli
   ant --version
```

### Create your own dweb App

You may want to clone Fileman and try that out before creating your own app.

For your own project, any web framework or tooling will do so long as you can use it to create a static website. The following shows how to create a web app using Svelte (as used in Fileman). Assuming you already have Node.js, create your Svelte project:

```bash
# create a new project in the current directory
npx sv create

# create a new project in my-app
npx sv create my-app
```

Once you've created your project (or cloned Fileman), install the dependencies with `npm install` (or `pnpm install` or `yarn`), and then start a development server:

```bash
npm run dev

# or start the server and open the app in a new browser tab
npm run dev -- --open
```

To do anything useful though, you need a network to talk to. See next.


### Develop your App
For details of the **RESTful HTTP API** provided by dweb see [github](https://github.com/happybeing/dweb/blob/main/dweb-cli/README.md#web-api). These APIs are very early stage and suggestions or help with extending and improving them are welcome.

Once you have a local test network and dweb server running, your app will be able to talk to the local Autonomi network using dweb APIs provided by the local dweb server. You can also try this using this example app.

### Test your App

- start a local testnent (see the [Autonomi Developer Docs](https://docs.autonomi.com/developers))
- start the dweb server with `dweb server --local`
- open your app (or this one) with `npm run dev -- --open`

You may also want to use either the Autonomi command line app (`ant`) or `dweb` to upload some data to the local network for testing.

Testing your app on the Autonomi public network is exactly the same, except you start the dweb server without using `--local`.

### Build your App

To create a production version of your app:

```bash
npm run build
```

You can preview the production build with `npm run preview`.

> Before deploying your dweb-app will need to install an adapter for static site generation. See [Svelte adapters](https://svelte.dev/docs/kit/adapters).

After building this as a static app you can use `dweb` to publish it on your local testnet, and eventually on the public main net. See the dweb [github](https://github.com/happybeing/dweb/blob/main/dweb-cli/README.md#dweb-command-line-app) for more about this.

## Contributions
Contributions under the AGPL3.0 license are welcome and any contributions or PRs submitted will be assumed to be offered under that license unless clearly and prominently specified otherwise. Any contributions are accepted on the condition they conform to that license and the following conditions:

- that by submitting a contribution you are confirming that you are the sole author, understand all the submitted code in depth, and that no AI or other code generation tool that has ingested copyright material was used in generating content in your contribution.

Thanks for understanding that I don't want to accept material of unknown provenance nor spend time reviewing code that the contributor doesn't understand completely.

## LICENSE

Everything is licensed under AGPL3.0 unless otherwise stated.

See also [./LICENSE](./LICENSE)
