---
name: Maud → Sailfish migration
description: Full HTML templating rewrite from maud macros to sailfish .stpl files; maud removed from the codebase
type: project
---

Completed a full migration from `maud` to `sailfish` for all HTML generation. Maud is no longer a dependency.

**Why:** maud's `html! {}` DSL was considered clunky, unreadable, and unmaintainable — deeply nested macro invocations interleaving Rust control flow with HTML. Sailfish uses `.stpl` files with EJS-like `<% %>` / `<%= %>` tags and real Rust expressions, which is more readable and editable as HTML.

**What changed:**
- `generator::Content::generate()` return type changed from `Result<maud::Markup, Error>` to `Result<String, Error>` — the shared boundary is now plain HTML strings
- `pages::*` functions return `axum::response::Html<String>` (axum sets the correct content-type header)
- `WeatherCode::as_img() -> Markup` renamed to `svg() -> &'static str`; same for `WindDirection`
- Icon SVGs (`iconify::svg!` calls) extracted to module-level `const` values in `weather.rs` so templates can reference them directly (sailfish generates code in the same module scope)
- `mashup.rs` uses `tokio::try_join!` to render left/right plugins in parallel

**Template locations:** `crates/server/templates/` with subdirs `pages/`, `weather/`, `mashup/`, `serve/`

**Sailfish 0.9 gotchas:**
- Struct fields must be accessed as `self.field` in templates (not bare `field`)
- `chrono::format()` returns `DelayedFormat` which doesn't implement `Render` — call `.to_string()` on it
- Custom types implementing `Display` don't automatically get `Render` — use `.to_string()` in templates
- Module-level constants defined in the same `.rs` file as the template struct are accessible directly in templates

**How to apply:** When adding new plugins or modifying existing HTML, write `.stpl` files in `crates/server/templates/`, define a `#[derive(TemplateOnce)]` struct, and return `String` from `generate()`.
