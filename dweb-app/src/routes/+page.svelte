 <!-- Copyright (c) 2025 Mark Hughes

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU Affero General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU Affero General Public License for more details.

 You should have received a copy of the GNU Affero General Public License
 along with this program. If not, see <https://www.gnu.org/licenses/>. -->

<script>
import { invoke } from "@tauri-apps/api/core";
import {onMount} from 'svelte';

onMount(async () => {
  console.log("onMount() starting dweb server...");
  await startServer();
});

let dwebHost = "http://127.0.0.1";
let dwebPort = 5537;
let dwebServer = dwebHost + ":" + dwebPort;
let dwebWebsite = "awesome";

async function startServer() {
  await invoke("start_server", {port: dwebPort});
}

async function browseAutonomi() {
  console.log("browseAutonomi()");
  invoke("dweb_open", {addressNameOrLink: dwebWebsite});
}
</script>

<main class="container">
  <h1>Autonomi dweb     <a href="https://codeberg.org/happybeing/dweb#dweb" target="_blank">
      <img src="/dweb-logo.svg" class="logo dweb" alt="dweb Logo" />
    </a>
</h1>

  <div class="row">
  </div>

  <p><button onclick={browseAutonomi}>Browse Autonomi dweb</button></p>
  <p>Explore the secure peer-to-peer web on the Autonomi network</p>
</main>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 40px;
  padding-left: .5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.dweb:hover {
  filter: drop-shadow(0 0 .3em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

button {
  outline: none;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>
