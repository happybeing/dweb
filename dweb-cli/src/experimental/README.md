### Experimental 'Hosts' UI

Prior to 'ports' based solution I tested a approach using 'hosts' based URLs. This delivers more familiar style domain like addressing in the browser address bar, but requires more complex setup: e.g. a local DNS server.

An alternative to a local DNS would be to have the dweb server act as a proxy for all web access. This has not been explored for lack of time and the fact that it too would require additional setup, albeit less than using a local DNS.

So in order to avoid the need for additional setup, the 'hosts' based approach has been replaced with a 'ports' based solution (a server on a different port per website).

For now, dweb retains the earlier 'hosts' solution under control of the `--experimental` command line option for relevant subcommands.

## Example Use
Before proceeding you will need to carry out additional setup as described under 'Hosts / Experimental Setup'.

Once you have the local DNS setup, start the experimental (with hosts) server:

```
dweb serve --experimental
```

View a website, and notice the address bar shows a human readable domain (unlike the regular server which shows '127.0.0.1'):

```
dweb open --experimental awesome
```

The above 'with hosts' web addresses are similar to `awe`, an earlier demonstration app that showed how a custom browser might work, with direct access to Autonomi. `awe` now uses `dweb` for its implementation and so remains a useful demonstration (see [github.com](https://github.com/happybeing/awe)).

## Hosts / Experimental Setup
Below is a draft of the setup documentation for the earlier host based solution, which is still available to be tried out using the `-- experimental` option on relevant dweb CLI subcommands.

After installing dweb you must set-up local forwarding of dweb domains to http://localhost as follows.

   **Windows:**
   I have not tested a solution for Windows yet. One possible solution is using [marlon-tools](https://github.com/hubdotcom/marlon-tools#marlon-tools) to simple DNS proxy server (after installing Python).

   The forward "*.au" to 127.0.0.1 with:
   ```
   127.0.0.1 *.au
   ```
   That is a first test but not a good idea as it will intercept all Australian websites! So if that works, try changing it to:
   ```
   127.0.0.1 *-dweb.au
   ```

   If the first one works but not the second, then try:
   ```
   127.0.0.1 *.web-dweb.au
   127.0.0.1 *.api-dweb.au
   127.0.0.1 *.app-dweb.au
   ```

   If any of this works (on Windows, Linux or Mac) please open an issue to let me know. Also, if you find another way!

   **MacOS:**
   I haven't tested a solution for Mac yet so if either of the solutions for Windows or Linux work, please open an issue to let me know.

   **Linux:**
   For testing the hosts approach on Ubuntu, I used `dnsmasq`. To set this up, follow the instructions [here](https://help.ubuntu.com/community/Dnsmasq) or the tl;dr below:
   ```
   # Edit /etc/systemd/resolved.conf and set DNS=127.0.0.1

   # Then:
   sudo systemctl restart systemd-resolved

   sudo apt update
   sudo apt install dnsmasq
   sudo echo "address=/*-dweb.au/127.0.0.1" >> /etc/dnsmasq.conf
   address=/*-dweb.au/127.0.0.1
   sudo systemctl enable dnsmasq
   ```

