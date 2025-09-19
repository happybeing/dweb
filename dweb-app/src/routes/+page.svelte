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
  await refreshWallet();
});

let dwebHost = "http://127.0.0.1";
let dwebPort = 5537;
let dwebServer = dwebHost + ":" + dwebPort;
let dwebWebsite = "awesome";

// Wallet info state
let walletAddress = "";
let antBalance = "";
let ethBalance = "";
let walletLoading = false;
let walletError = "";

// Open by address state
let openAddress = "";

function formatAmount(amount) {
  if (!amount) return "";
  const s = String(amount);
  const parts = s.split(".");
  if (parts.length === 1) return parts[0];
  const intPart = parts[0];
  const fracPart = parts[1] || "";
  const shown = fracPart.slice(0, 5);
  const needsDots = fracPart.length > 5;
  return `${intPart}.${shown}${needsDots ? "…" : ""}`;
}

async function startServer() {
  try {
    await invoke("start_server", {port: dwebPort});
  } catch (e) {
    console.error("Failed to start server", e);
  }
}

async function browseAutonomi() {
  console.log("browseAutonomi()");
  invoke("dweb_open", {addressNameOrLink: dwebWebsite});
}

async function openByAddress() {
  const addr = (openAddress || "").trim();
  if (!addr) return;
  invoke("dweb_open", {addressNameOrLink: addr});
}

/** @param {KeyboardEvent} ev */
function handleEnter(ev) {
  if (ev.key === 'Enter') {
    openByAddress();
  }
}

async function refreshWallet() {
  walletLoading = true;
  walletError = "";
  try {
    // Retry a few times in case the server isn't ready yet
    const url = `${dwebServer}/dweb-0/wallet-balance`;
    for (let i = 0; i < 5; i++) {
      try {
        const res = await fetch(url);
        if (res.ok) {
          const data = await res.json();
          walletAddress = data.wallet_address || "";
          antBalance = data.ant_balance || "";
          ethBalance = data.eth_balance || "";
          walletLoading = false;
          return;
        }
      } catch (_) {}
      await new Promise(r => setTimeout(r, 500));
    }
    walletError = "Unable to fetch wallet info";
  } finally {
    walletLoading = false;
  }
}
</script>

<main class="container">
  <div class="topbar">
    <div class="wallet center" title="Current wallet balances">
      {#if walletLoading}
        <span>Loading wallet…</span>
      {:else if walletError}
        <span>{walletError}</span>
      {:else}
        <span class="wallet-item"><strong>Wallet</strong>: {walletAddress}</span>
        <span class="wallet-item" title={antBalance}><strong>ANT</strong>: {formatAmount(antBalance)}</span>
        <span class="wallet-item" title={ethBalance}><strong>ETH</strong>: {formatAmount(ethBalance)}</span>
      {/if}
    </div>
  </div>

  <h1>Autonomi dweb     <a href="https://codeberg.org/happybeing/dweb#dweb" target="_blank">
      <img src="/dweb-logo.svg" class="logo dweb" alt="dweb Logo" />
    </a>
</h1>

  <div class="row">
  </div>

  <p><button onclick={browseAutonomi}>Browse Autonomi dweb</button></p>
  <p>Explore the secure peer-to-peer web on the Autonomi network</p>

  <div class="open-input">
    <input
      type="text"
      bind:value={openAddress}
      placeholder="Open dweb app via address or name"
      onkeydown={handleEnter}
      aria-label="Open dweb app via address or name"
    />
    <button onclick={openByAddress}>Open</button>
  </div>
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

.topbar {
  position: fixed;
  top: 0.75rem;
  left: 0;
  right: 0;
}

.wallet {
  display: flex;
  gap: 0.75rem;
  font-size: 0.85rem;
  align-items: center;
}

.wallet.center {
  justify-content: center;
}

.wallet-item {
  white-space: nowrap;
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

.open-input {
  margin-top: 1rem;
  display: inline-flex;
  gap: 0.5rem;
  align-items: center;
  justify-content: center;
}

.open-input input[type="text"] {
  padding: 0.55em 0.8em;
  border: 1px solid #ccc;
  border-radius: 8px;
  min-width: 22rem;
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
