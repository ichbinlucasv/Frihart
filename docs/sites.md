# Sites we claim

Frihart claims **documents**, not “the web”. A named host lands here
only when a person has opened it in Frihart and it is readable without
JavaScript. The in-browser list is `about:sites`.

## Internal (always)

- `about:home`, `about:engine`, `about:campaigns`, `about:processes`,
  `about:linux`, `about:script`, `about:sites`

These are our chrome. They do not prove the HTML engine.

## Claimed (opened in this tree)

| URL | Why |
| --- | --- |
| https://example.com/ | Live HTML 2026-08-18. Title, `h1` at `1.5em` of parent, two paragraphs, IANA link via `a:link`. Body `60vw` + `margin: auto` centers the column. Light `#eee` canvas gets dark ink so it stays readable. `opacity` and `font-family` are ignored. |
| https://www.rfc-editor.org/rfc/rfc1918.html | Live HTML 2026-08-18. No `<title>` — title from `span.h1`. Nine `<pre>` pages + `<hr>` page breaks. Private nets `10/8`, `172.16/12`, `192.168/16` visible. Also checked at 5120×1440 (G9-class). |
| https://suckless.org/ | Live HTML 2026-08-18. Title, News, dwm/dmenu. Protocol-relative `//` links become https. No script. Also 5120×1440. |
| https://www.gnu.org/philosophy/ | Live HTML 2026-08-18. Title, “four essential freedoms”, essay list, `/philosophy/free-sw.html` links. `&mdash;` / `&ldquo;` decode. `@media` two-column CSS ignored (stacks). No script. Also 5120×1440. |
| https://www.kernel.org/ | Live HTML 2026-08-18. Title, About/Releases nav as clickable list links, releases table (`mainline` / `stable` / `7.2` cells). Download URLs in cells are text, not separate hits. External CSS not loaded (mobile header also shows). IE conditional scripts skipped. Also 5120×1440. |
| https://docs.kernel.org/ | Live HTML 2026-08-18. Sphinx index. Title, “The Linux Kernel documentation” without permalink `¶`, toctree links (`process/development-process.html`). `display:none` search box stays hidden (no JS). Sidebar stacks above the body (no Alabaster CSS). Also 5120×1440. |
| https://www.ietf.org/ | Live HTML 2026-08-18. Title, Welcome, IETF 126 Vienna, IETF 127 San Francisco as a heading-link, standards paragraph. Bootstrap megamenu stacks (no CSS). Scripts skipped. Also 5120×1440. |
| https://www.rfc-editor.org/ | Live HTML 2026-08-18 (same bytes as the live fetch). Nuxt SSR index. Title, “The official home of RFCs”, Latest RFCs as one hit each (`RFC 10030: Network Time Protocol…` → `/info/rfc10030/`). UTF-8 text (nbsp between RFC and number) is not Latin-1-mangled. SVG icons skipped. Tailwind unused (nav stacks). Also 5120×1440. |
| https://www.w3.org/ | Live HTML 2026-08-18. Title `W3C`, h1 “Making the web work”, Consortium lead with `/standards/` and mission links, TPAC 2026, Web standards, Latest news “ARIA in HTML” as a heading-link. External CSS unused (nav stacks). Font-loader scripts skipped. Also 5120×1440. |
| https://www.w3.org/TR/ | Live HTML 2026-09-14. Title / h1 “W3C standards and drafts”. Lead on technical reports. “Showing 1236 reports across 288 families.” Family `h2` (RDF, …) and spec `h3` heading-links (`RDF 1.2 Turtle` → `/TR/rdf12-turtle/`). Maturity (`Draft Standard`) and dates as text. Tags / Deliverers via HTML5 `dl` `div` wrappers. Filter form is GET (JS unused). External CSS unused (nav stacks). Also 5120×1440. |
| https://www.w3.org/TR/webarch/ | Live HTML 2026-09-14. Title / h1 “Architecture of the World Wide Web, Volume One”. W3C Recommendation 15 December 2004. Abstract (identification / representation / protocols). Sections Identification, Interaction, Data Formats. `dt`/`dd` that is one link (“This version” / “Latest version”) is a hit. `dfn` / `acronym` stay in the text. Named-anchor headings are headings, not fake links. External CSS unused. Also 5120×1440. |
| https://www.rfc-editor.org/rfc/rfc9110.html | Live HTML 2026-09-14. Title `RFC 9110: HTTP Semantics`. Abstract, Status of This Memo, Methods/GET, idempotent/safe. `pre` ABNF kept. rfc-editor links. External CSS unused. Also 5120×1440. Layout of this document is slow (large HTML + many `pre`); that is honest, not a skip. |
| https://www.w3.org/TR/WCAG22/ | Live HTML 2026-09-14. Title / h1 “Web Content Accessibility Guidelines (WCAG) 2.2”. W3C Recommendation 12 December 2024. Abstract, Perceivable / Operable / Understandable / Robust. Guideline 1.1 Text Alternatives. This-version `dt`/`dd` link is a hit. External CSS unused. Also 5120×1440. |
| https://www.rfc-editor.org/rfc/rfc8446.html | Live HTML 2026-09-14. Title from `span.h1`: “The Transport Layer Security (TLS) Protocol Version 1.3”. Abstract, Handshake/ClientHello, `pre` pages (same shape as RFC 1918). Also 5120×1440. |
| https://www.rfc-editor.org/rfc/rfc5280.html | Live HTML 2026-09-14. Title from `span.h1`: “Internet X.509 Public Key Infrastructure Certificate…”. Abstract, Certificate/CRL, `pre` pages. Also 5120×1440. |
| https://www.rfc-editor.org/rfc/rfc8032.html | Live HTML 2026-09-22. Title from `span.h1`: “Edwards-Curve Digital Signature Algorithm (EdDSA)”. Abstract, Ed25519 / edwards25519, `pre` pages (same shape as RFC 1918/8446/5280). Also 5120×1440. |
| https://www.rfc-editor.org/rfc/rfc7748.html | Live HTML 2026-09-22. Title from `span.h1`: “Elliptic Curves for Security”. Abstract, Curve25519 / X25519, `pre` pages (same shape as RFC 1918/8446/5280/8032). Also 5120×1440. |
| https://www.rfc-editor.org/rfc/rfc5869.html | Live HTML 2026-09-22. Title from `span.h1`: “HMAC-based Extract-and-Expand Key Derivation Function (HKDF)”. Abstract, HKDF-Extract / HKDF-Expand, `pre` pages (same shape as RFC 1918/8446/5280/8032/7748). Also 5120×1440. |
| https://www.openbsd.org/ | Live HTML 2026-09-22. Title `OpenBSD`. About / Project Goals / Security / Crypto nav. Callout “Only two remote holes…”. FREE / OpenSSH / OpenBSD 7.9. External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.openssh.org/ | Live HTML 2026-09-22 (`openssh.com` 301 → `openssh.org`). Title `OpenSSH`. About / Project Goals / Security nav. Callout OpenSSH 10.5. ssh / scp / sftp suite. External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.libressl.org/ | Live HTML 2026-09-22 (`libressl.org` 301 → `www.libressl.org`). Title `LibreSSL`. About / Project Goals / Papers nav. Callout LibreSSL 4.3.2. libcrypto / libssl / libtls. External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.openbsdfoundation.org/ | Live HTML 2026-09-22 (`openbsdfoundation.org` 301 → `www.openbsdfoundation.org`). Title `The OpenBSD Foundation`. Funding for OpenBSD and related projects. Bylaws / Donations / Activities / Contributors / Membership nav. 2026 Fundraising Campaign. Bootstrap CSS unused (nav stacks). Footer jQuery/bootstrap scripts skipped. Also 5120×1440. |
| https://www.openbgpd.org/ | Live HTML 2026-09-22 (apex and www same bytes). Title `OpenBGPD`. About / Project Goals / Manual Pages / Presentations nav. FREE Border Gateway Protocol Version 4. External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.opensmtpd.org/ | Live HTML 2026-09-22 (apex and www same bytes). Title `OpenSMTPD`. About / Project Goals / Manual Pages / Presentations nav. Callout OpenSMTPD 7.8. FREE server-side SMTP (RFC 5321). External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.openntpd.org/ | Live HTML 2026-09-22 (apex and www same bytes). Title `OpenNTPD`. About / Project Goals / Manual Pages / Presentations nav. Callout OpenNTPD 7.9p1. FREE Network Time Protocol. External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.openiked.org/ | Live HTML 2026-09-22 (apex and www same bytes). Title `OpenIKED`. About / Project Goals / Manual Pages / Presentations nav. Callout OpenIKED 7.4. FREE IKEv2 / Internet Key Exchange (IPsec VPN). External CSS unused (nav stacks). No script. Also 5120×1440. |
| https://www.bearssl.org/ | Live HTML 2026-09-22 (apex and www same bytes). Title `BearSSL - Main`. Overview / Status / Download and Installation / Features. SSL/TLS in C (RFC 5246), constant-time crypto, no `malloc`. External CSS unused (nav stacks). IE conditional scripts skipped. Also 5120×1440. |
| https://www.wireguard.com/ | Live HTML 2026-09-22 (apex and www same bytes). Title `WireGuard: fast, modern, secure VPN tunnel`. Installation / Quick Start / Protocol & Cryptography / Donate nav. Conceptual Overview, Cryptokey Routing, Simple Network Interface. External CSS unused (nav stacks). `scripts.min.js` skipped. Also 5120×1440. |

## Target (open next)

Twenty-eight public claims are in. CSS leftovers `list-style` /
`white-space` are in. wireguard.com claimed this session. Named queue
empty. Suggest next static OPSEC homepage: gnupg.org (0 scripts on
probe), or curl.se / noiseprotocol.org if still static/readable. Arch
package builds; install is `sudo pacman -U` on the user's machine.

## Will not claim

| URL | Why |
| --- | --- |
| Wikipedia | Infobox / more CSS first |
| GitHub, Gmail, maps, banks, social | JS apps |
| Proton Mail | JS app (Proton Pass is an external manager) |

A missing JS app is expected. A broken static layout is a bug.

See [engine.md](engine.md) and [css-subset.md](css-subset.md).
