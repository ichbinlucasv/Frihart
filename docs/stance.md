# Product stance

How Frihart stays a LibreWolf-ethic browser without pretending to be
Chrome, and without pretending I2P is the whole web.

Read with [PHILOSOPHY.md](../PHILOSOPHY.md) and [docs/opsec.md](opsec.md).

> **Update 2026-10-13:** Frihart is now **hybrid**. Servo is embedded as the
> clearnet *Compat* engine; Frihart's own engine keeps Tor, I2P and reader
> tabs. See [docs/ENGINE_HYBRID.md](ENGINE_HYBRID.md).

## Compatibility floor (not CERN, not Blink)

We do **not** rebuild the web from the 1990 CERN line-mode client.
That is museum work. It does not help anyone read today's documents.

The engine floor is **document HTML** still in use: roughly HTML 4 / early
CSS (the 1996–2005 shape that RFCs, manuals, homepages, and many 2020s
static pages still have). Campaign D claims those live documents, one
host/path at a time.

The ceiling for the next years is **not** “every modern site.” A
general-purpose engine that matches Blink is a decade even with a
company. Frihart’s bet is **harder to own**.

| We do | We do not |
| --- | --- |
| Grow the subset until docs, RFCs, blogs, and homepages are a daily driver | Reimplement Mosaic, Netscape 4, Flash, Java applets, ActiveX |
| Fail clearly when a page needs JS (own engine) | Embed V8 / SpiderMonkey / QuickJS into the own engine |
| Offer Servo as a clearnet Compat engine (see ENGINE_HYBRID.md) | Let Servo handle Tor or I2P tabs |
| Add CSS the claimed documents actually use | Fake flex/grid so a page “kind of” works |
| Keep JS off until a tiny Frihart interpreter (Campaign G, later) | Chase Gmail, maps, banks, or social |

Sites from 1996 that are still documents are in scope. SPAs that are
applications are out of scope until G exists — and even then they are
not a promise.

## Two modes, one binary

**Anonymous by default on the clearnet is a lie.** I2P is a separate
network. Tor as the only circuit breaks daily-driver docs and fights
Tails (which already owns “everything is Tor”).

| Mode | What it is | Default? |
| --- | --- | --- |
| **Sovereign** | LibreWolf stance on clearnet: HTTPS-only, partitioned cookies, native blocker, frozen UA, no telemetry, no account | **Yes** |
| **Anonymous circuit** | Tor tab or I2P tab. System daemon. SOCKS only. **Fail closed** (no clearnet fallback) | Opt-in (`--tor`, `--i2p`, Ctrl+Shift+O / I) |

Private windows are local amnesia (memory profile). They are not
anonymity. Tor/I2P are anonymity plus a circuit.

We do not bundle Tor or I2P. We do not start a second daemon on Tails.

The first audience is the people who already live this way — Monero,
I2P, Tor, GnuPG, SimpleX, Session, Meshtastic, Tails, Qubes. The
browser joins that stack. It does not replace it. Details:
[community.md](community.md).

**Hidden names:** `.onion` is Tor-tab only. `.i2p` is I2P-tab only.
Direct tabs never resolve those labels (no system DNS leak). I2P tabs
refuse clearnet; use Tor if you need anonymous public HTTP.

## Money

Linux stays **free**. There is no ad tier, no BAT, no sponsored search.

Supporter plan (draft): **€4.99/month, €49/year or €149 lifetime**
(launch: €99 lifetime for the first 500 supporters). Bundle with HashChat:
€7.99/month or €229 lifetime. Supporter adds builds for other OSes, themes,
advanced profiles/containers, extra lists and priority support. Local
Ed25519 key, checked offline, no license server, no Frihart account. Crypto
only: Monero preferred, Bitcoin and Lightning welcome.

Anyone may **voluntarily** send any amount (pay-what-you-want) to
support Linux work. Payment never unlocks a privacy feature. Privacy is
not a product tier. See [pricing.md](pricing.md) and [SUPPORT.md](SUPPORT.md).

## DNS and malware

- Clearnet tabs (Servo Compat and the own engine) resolve with **Quad9
  DNS-over-HTTPS** (`https://dns.quad9.net/dns-query`). The connection is
  pinned to Quad9's published addresses, so the name is not looked up on
  the system resolver.
- The system resolver is **opt-in**: `network.doh_mode = system`, or
  `network.dns_system_fallback = true` for when Quad9 does not answer.
  A block or NXDOMAIN does not fall through.
- Tor and I2P tabs never use local DNS. Names go to SOCKS. `.onion` and
  `.i2p` are refused before any clearnet lookup. Servo refuses those
  names itself; the own engine still opens them on the matching tab.
- On Tails, the system resolver is Tor. A clearnet tab on Quad9 asks
  Quad9 outside Tor. Opt into the system resolver there if that tab
  should follow Tails.
- Malware blocking in-browser is the **native blocker**, lists local,
  no Frihart list server, no Google. Quad9's malware filtering is the
  resolver, not a Frihart list.

## Quantum

A quantum computer does not “find people in Frihart” if Frihart never
collected them.

What actually helps:

1. **Collect nothing.** No telemetry, no account, no crash ping. A
   future decrypt of our “servers” finds empty, because there are no
   Frihart servers.
2. **Follow rustls** for post-quantum TLS (hybrid KEM) when it is
   stable. We do not write our own PQ crypto.
3. **Tor/I2P** hide *who talked to whom* from the local network. They
   are not a proof against a cryptographically-relevant quantum
   adversary on those networks. We will not claim they are.
4. **No identifiers to merge.** Frozen UA, no Client Hints, partitioned
   state, no login vault.

We will not ship a “quantum anonymous” badge. Honesty is the feature.

## What we implement next (this tree)

1. Document this stance (this file).
2. I2P tabs that fail closed, same shape as Tor.
3. Clearnet DNS is Quad9 DoH. System DNS is opt-in
   (`network.doh_mode = system` or `network.dns_system_fallback`).
4. Keep claiming static documents (Campaign D). Then CSS leftovers.
   Then an Arch package that actually installs (Campaign F).
5. Campaign G (script) stays refuse. H/I stay parked.
