# frihart-own

Frihart's own Rust engine. It lives here so it can replace Servo and Boa
later. The browser does not call this crate yet.

`READY_TO_REPLACE_COMPAT` is false. Layout uses the existing pipeline.
`eval_script` runs a small arithmetic language written in this folder.
That language is not JavaScript. Boa, behind the `boa` feature of
`frihart-js`, is the light JS bridge until this folder can do the same
job. Servo stays the clearnet Compat engine until
`content.own_replaces_compat` is turned on.

See [ROADMAP.md](../../ROADMAP.md) Campaign J and
[docs/ENGINE_HYBRID.md](../../docs/ENGINE_HYBRID.md).
