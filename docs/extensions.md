# Extensions and the community

Frihart will have extensions. They will not look like the Chrome Web
Store or AMO.

## What we will do

- Publish a small, documented API when the engine can isolate a
  content process (Phase 6+) and bind script (Phase 7+).
- Load extensions from the user's profile (`extensions/` as local
  directories). No remote gallery that we host.
- Welcome community add-ons as **separate open-source repos** people
  can clone and load. Codeberg is the natural home.
- Review security-sensitive APIs the way we review the blocker.

## What we will not do

- Ship a "recommended extensions" feed from Frihart servers
- Load unsigned remote blobs
- Pretend we are Firefox so existing `.xpi` files just work
- Give an extension chrome-process rights

## Until the API exists

Containers, the native blocker, DeepL, Swisscows/DDG, Tor tabs, and
VPN hooks live **in the browser**. That is intentional. Those jobs
should not wait on an add-on store.

If you want to help, open an issue or a pull request on Codeberg.
Read [CONTRIBUTING.md](../CONTRIBUTING.md).
