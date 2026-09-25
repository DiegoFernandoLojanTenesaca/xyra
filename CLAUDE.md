# Xyra: internal guide

Windows desktop companion for League of Legends. It reads the augment cards on screen with the Windows OCR, rates them
with OP.GG and draws labels on a native Direct2D overlay. Rust + Tauri 2 + Svelte 5.

## Architecture

| Path | Owns |
|---|---|
| `crates/xyra-core/` | Platform-independent logic: card detection and rating (`cards`), OP.GG (`opgg`), League client and live game (`lol`), catalog, stats, profile, config, build import (`import`), official game options (`game_settings`), i18n and design tokens (`theme`). Reusable by a future mobile companion. |
| `crates/xyra-core/src/model.rs` | Every type the UIs see. `cargo test` exports them to `src/lib/generated/` with ts-rs. |
| `shared/locales/<language>/<namespace>.json` | All user-facing text. Read by the UI (i18next) and by Rust (`i18n::t`) for the overlay, tray, voice, CSV and item sets. |
| `shared/tokens.json` | Colors, spacing, type and size scales. The UI publishes them as CSS variables (`theme.ts`); the overlay reads them with `theme::color`. |
| `src-tauri/src/engine.rs` | Engine loop (client phase, live game, OCR, overlay) and the state shared with commands. |
| `src-tauri/src/commands.rs` | Thin Tauri commands delegating to the engine and the core. |
| `src-tauri/src/overlay.rs` | Layered click-through window drawn with Direct2D; the 5 label styles. |
| `src/lib/app.svelte.ts` | UI state and actions; pages import it instead of receiving props. |
| `src/lib/components/` | Reusable UI primitives. Pages only compose them. |

## How to extend

- **New text:** add the key to every language in `shared/locales/*/<namespace>.json` (the `every_language_has_the_base_keys` test fails otherwise). Logic returns keys, screens render `t(key)`.
- **New language:** add a folder under `shared/locales/` with the same files. No code change.
- **New color or size:** add it to `shared/tokens.json` and use `var(--group-name)` in CSS or `theme::color("group.name")` in Rust.
- **New data for the UI:** add the field in Rust, run `cargo test --workspace`, import the generated type from `src/lib/types.ts`.
- **New official game option:** add the `(section, key)` pair to `game_settings::ALLOWED` and its texts under `game:settings`.
- **Label previews:** `xyra.exe --previews docs/cards-background.png <out> es`, then crop into `static/styles/<style>.jpg`.

## Hard rules

- Never read or write game memory, inject, open the game process, simulate input, automate gameplay or reveal hidden information (see `SECURITY.md`).
- The client is only written when the player asks (rune page, item set) or enables an automation that uses official client options.
- No custom in-game timers: Riot bans enemy ultimate timers and may ban others; enable the game's own minimap timers instead.
- Code is English, with no comments beyond one-line docstrings for constraints the code cannot express.
- The log records only failures a human must look at.
- Commits follow conventional commits in lowercase; never add attribution footers.

## Commands

```powershell
pnpm install
pnpm tauri dev
cargo test --workspace
pnpm check
pnpm tauri build
```