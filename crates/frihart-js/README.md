# frihart-js

Home of Frihart's JavaScript story on the **own** engine.

Today this crate is the **refuse-first** host-API and value/opcode layer
(Campaign G). Untrusted script never runs. Fingerprint, storage and
network surfaces stay denied even if a JS pref is flipped.

## Campaigns that grow here

| Campaign | Role |
| --- | --- |
| **G Script** | Refuse layer and typed host APIs (`HostApi`). Stays. |
| **B-JS** | Optional Boa embed (feature-gated), wired through this crate's policy. Lives behind a flag; not the end state. |
| **J Own JS engine** | 10-year goal. The code lives in [`frihart-own`](../frihart-own), not here. This crate keeps the host-API refuse policy that both Boa and that engine must pass through. |

See [ROADMAP.md](../../ROADMAP.md) (Campaigns B-JS and J) and
[docs/ENGINE_HYBRID.md](../../docs/ENGINE_HYBRID.md).

## Module plan (Campaign J)

Planned modules as J lands (names may shift):

```
frihart-js/
  src/
    lib.rs          # public surface, HostApi refuse policy
    value.rs        # JS values (exists today as Value)
    opcode.rs       # bytecode ops (exists today as Opcode)
    host.rs         # HostApi + capability checks
    runtime.rs      # Runtime stub → real engine entry
    parser/         # lexer, parser, AST (J year 1)
    bytecode/       # compiler + interpreter + GC (J year 2)
    jit/            # baseline / optimizing tiers (J year 6+)
    builtins/       # ES builtins
    test262/        # harness hooks for CI
```

Boa (B-JS), if enabled, is a separate optional dependency and must call
into `HostApi` / `frihart-net` the same way the own engine will. When J
covers B-JS milestones M1–M4, Boa is removed.

## Rules

- No untrusted exec until a campaign milestone explicitly turns it on.
- Pref flip ≠ capability grant.
- Tor / I2P tabs stay refuse until a leak audit passes.
- 100% Rust in the end state (no SpiderMonkey, no V8, no Boa).
