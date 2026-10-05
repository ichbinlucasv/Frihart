# Privacy

Frihart does not collect personal data. There is no account, no sign-in, no telemetry, no crash reporting, no update check and no server run by this project that the browser talks to. The project cannot see what you browse because it has nowhere to receive it. We never ask for a phone number, email address, real name or identity document, in the browser or in any licence or payment step.

## What the browser contacts

Only what you ask for: the sites you open, and the DNS, proxy or VPN you set up yourself. Startup makes no connections. Tor and I2P tabs go through your own system daemons and refuse to fall back to a direct connection. DoH is off unless you give your own URL. The translator and search contact their providers only when you use them; the provider is shown in the settings.

## What is stored on your machine

Your profile directory holds preferences, bookmarks, history (off in private windows and if you turn it off), cookies, downloads metadata, containers, installed add-ons and identity autofill. Files are readable only by your user. They are not encrypted at rest yet; an encrypted profile is on the roadmap. Nothing is uploaded. There is no password store.

## Wipe, shred and panic

Wipe clears the session. Shred overwrites this profile's files and removes them. Ctrl+Shift+Backspace shreds the profile and quits without asking, and `frihart --wipe` does the same from a terminal. Overwriting does not reliably reach data on SSDs or copy-on-write filesystems, and files you downloaded to another folder are not touched.

## Your rights under GDPR and LGPD

Because the project holds no data about you, there is nothing for it to export, correct or erase on request. Your data is in your profile directory, where you can read it, copy it or shred it. Any future paid-build or donation flow is meant to need no account and record nothing beyond what the blockchain shows (Monero or Bitcoin by default, Lightning also welcome), and a data-processing statement will accompany any component that handles data for you.

## Contact

Questions about this document can be raised as an issue in the Codeberg repository. Do not send personal data there.
