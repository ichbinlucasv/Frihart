# Pricing

Linux desktop is **free**. Privacy is not a paid tier.

Every other OS (when it exists) is a one-time **€100** lifetime fee —
same model as HashChat Android. Pay with Monero or Bitcoin; Lightning is
welcome too. No subscription. No Frihart account. No ads. No Brave-style attention market.

**Mobile priority (after Linux desktop quality):** GrapheneOS, Jolla
(Sailfish), Volla Phone OS, and similar alternative / Linux-leaning phones
are first-class targets and must work great (stable + fast). Packaged
mobile builds are not the free desktop Linux binary; they are **€100
lifetime** like other non-Linux ports. GrapheneOS is Android-derived —
treat it as a priority phone port, not free desktop Linux.

Linux users may **voluntarily** send the same amount to fund the
project. That is support, not a license. It does not unlock features
the free build lacks.

| Platform | Price |
| --- | --- |
| Linux desktop | free |
| GrapheneOS / Jolla / Volla (packaged mobile) | €100 lifetime |
| Android (stock / other) | €100 lifetime |
| Windows | €100 lifetime |
| macOS | €100 lifetime |
| anything else | €100 lifetime |

Paid builds unlock with a **local** key. No license server.

The payment flow collects nothing beyond what the blockchain shows: no
name, email, phone number, country or ID is requested, and none is needed
to receive a key. The planned scheme is a fresh address per order and a
one-time key (a signature over a random order number) that the build checks
offline and that says nothing about the buyer.

Non-Linux builds are not sold to governments, state bodies or their
agencies; the purchase terms say so and the seller may refuse or refund a
sale. The source is MIT or Apache-2.0 and cannot be restricted, so this
covers sales and support only. We do not collect identity data to enforce
it, so a buyer who misstates who they are cannot be detected.

## Pay

Set addresses in `prefs.toml` (do not commit live wallets):

```toml
[support]
xmr = ""
btc = ""
fiat_url = ""
```
