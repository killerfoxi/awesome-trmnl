---
name: Plugin dynamism options
description: Trade-off analysis for making plugins dynamically loadable; subprocess model recommended
type: project
---

Discussed making plugins more dynamic (currently statically compiled into the binary via the `Plugin` enum in `plugins.rs`). No implementation was started.

**Options evaluated:**

1. **Subprocess** — each plugin is any executable writing HTML to stdout; server spawns it per request. `Content` trait already returns `String` so the adapter is ~20 lines. Language-agnostic, secrets in env vars, no ABI issues. Overhead is fine at 30-min TRMNL refresh intervals.

2. **Rhai (embedded scripting)** — pure-Rust scripting engine, plugins are `.rhai` files, reloadable without restart. Host exposes typed functions (`fetch_json`, etc.) plugins can call. More integrated than subprocess but Rhai is another language to learn.

3. **WASM (Wasmtime)** — sandboxed, cross-language, hot-loadable, real plugin ecosystem potential. Component model WIT defines the `generate() -> string` contract. Most setup work; async across the WASM boundary needs careful bridging.

4. **True dylibs (.so)** — advised against. Rust ABI is unstable across compiler versions, trait objects don't cross `.so` boundaries safely. `abi_stable` crate makes it workable but adds significant complexity for little gain over WASM.

**Recommendation:** Subprocess for immediate pragmatic win; Rhai if tighter integration (shared HTTP client, typed config) is needed later; WASM if a third-party plugin ecosystem is the goal.

**How to apply:** If the user asks to implement plugin dynamism, start with the subprocess adapter against the existing `Content` trait.
