# Who Frihart is for first

Paranoids, operators, and people who already run **Tor, I2P, Monero,
GnuPG, SimpleX, Session, Meshtastic**, Tails, and Qubes. Snowden-grade
habits: assume the network lies; assume quantum will decrypt old
bulk collection; do not be in that bulk.

Frihart is the **browser** in that stack. It is not the stack.

| You already use | In Frihart |
| --- | --- |
| Tor daemon | Tor tabs, fail closed. `.onion` only here. |
| I2P / i2pd | I2P tabs, fail closed. `.i2p` only here. No clearnet outproxy as default. |
| Monero | Donate addresses in `prefs.toml`. Launch an external wallet. **No built-in wallet.** |
| GnuPG / age | Detected on `about:stack`. Not a mail client. |
| SimpleX, Session | Launch if on PATH. Not embedded. |
| Meshtastic | Radio stays on the radio. Detected, not vendored. |
| Tails / Qubes | First-class homes. Amnesia and compartments win. |
| Quad9 | System DNS recommendation. Not forced DoH. |
| SearXNG | Your instance URL in `search.searxng`. We do not pick a public one. |

## Hard rules for this audience

1. **No DNS leak of hidden names.** A Direct tab that sees `.onion` or
   `.i2p` refuses before any resolver. That is the bug this community
   would never forgive.
2. **No Frihart servers** to subpoena, hack, or decrypt in 2040.
3. **No account, no telemetry, no Safe Browsing, no wallet, no chat
   protocol inside the binary.**
4. **Launch, don't embed.** SimpleX inside a browser is someone else's
   RCE plus our process model. Same for Session, Monero, Meshtastic.
5. **HTTP on `.onion` / `.i2p` is allowed** on the matching circuit.
   HTTPS-only is a clearnet rule. The circuit is the encryption.

See [stance.md](stance.md), `about:stack`, `about:tor`, `about:i2p`.
