# Xyra: internal guide

Windows desktop companion for League of Legends. It reads the augment cards on screen with the Windows OCR, rates them
with OP.GG and draws labels on a native Direct2D overlay; it also shows builds, runes, matchups and counters. Rust +
Tauri 2 + Svelte 5.

## Architecture

| Path | Owns |
|---|---|
| `crates/xyra-core/` | Platform-independent logic, reusable by a future mobile companion: card detection and rating (`cards`), OP.GG (`opgg`), the League client over HTTPS and WebSocket (`league`), client state parsing (`gameflow`, `champ_select`), `catalog`, `stats`, `profile`, `config`, rune and item set import (`client_import`), official game options (`game_settings`), every file on disk (`storage`), i18n and design tokens (`theme`), typed failures (`errors`). |
| `crates/xyra-core/src/model.rs` | The types the UI sees and the enums every layer shares. `cargo test` exports them to `src/lib/generated/` with ts-rs. |
| `crates/xyra-core/tests/live_client.rs` | Smoke test against the running client and OP.GG. |
| `locales/<language>/<namespace>.json` | All user-facing text. Read by the UI (i18next) and by Rust (`i18n::t`) for the overlay, tray, voice, CSV and item sets. |
| `design/tokens.json` | Colors, spacing, type, tracking, borders and sizes. The UI publishes them as CSS variables (`design/theme.ts`); the overlay reads them with `theme::color`. |
| `src-tauri/src/engine/` | Event loop on its own thread: client session (`mod.rs`), champion select with counter picks and automatic runes (`champ_select.rs`), card reading (`card_reader.rs`), lockfile and game.cfg watching (`watch.rs`). |
| `src-tauri/src/commands.rs` | One-line Tauri commands delegating to `Shared` and the core. |
| `src-tauri/src/overlay.rs` | Click-through layered window drawn with Direct2D on its own message-loop thread; the 5 label styles. |
| `src-tauri/src/riot_install.rs` | Where League is installed and its locale (Windows paths). |
| `src/lib/services/` | The only place that calls `invoke`: one typed function per command, plus `resource()` for loads that must drop stale answers. |
| `src/lib/app.svelte.ts` | UI state, pages and settings tabs; screens import it instead of receiving props. |
| `src/lib/screens/`, `src/lib/ui/` | Screens compose the catalogued primitives in `ui/`. |

## How it stays up to date

Push, not poll. The engine subscribes to the client WebSocket (gameflow session, champion select session, current
summoner, end of game) and watches the lockfile and `game.cfg`. The only timed work is reading the screen during a game
with augments, because nothing announces the cards; it stops as soon as the game ends.

## How to extend

- **New text:** add the key to every language in `locales/*/<namespace>.json` (the `every_language_has_the_base_keys` test fails otherwise). Logic returns keys or enums, screens render `t(key)`; counts go through `{{count, number}}`, numbers and dates through `app.format`.
- **New language:** add a folder under `locales/` with the same files. No code change.
- **New color or size:** add it to `design/tokens.json` and use `var(--group-name)` in CSS or `theme::color("group.name")` in Rust.
- **New command:** a function in `commands.rs` that delegates, registered in `lib.rs`, and its typed wrapper in `src/lib/services/`.
- **New data for the UI:** add the field in Rust, run `cargo test --workspace`, import the generated type from `src/lib/types.ts`.
- **New failure:** a variant in `errors::AppError` and its text in `locales/*/errors.json`.
- **New official game option:** a variant in `game_settings::GameOption` with its client location and texts under `game:settings`.
- **Label previews:** `xyra.exe --previews docs/cards-background.png <out> es`, then crop (330, 130, 1590, 900) to 960×587 into `static/styles/<style>.jpg`.

## Hard rules

- Never read or write game memory, inject, open the game process, simulate input, automate gameplay or reveal hidden information (see `SECURITY.md`).
- The client is only written when the player asks (rune page, item set) or enables an automation that uses official client options.
- Local connections verify the client certificate against `riotgames.pem`; internet requests use the system verification. Never accept invalid certificates.
- No custom in-game timers: Riot bans enemy ultimate timers and may ban others; enable the game's own minimap timers instead.
- Never recommend a champion the account cannot play (`ChampionInfo.locked`).
- Code is English, with no comments beyond one-line docstrings for constraints the code cannot express.
- The log records only failures a human must look at; persisted files that cannot be read are set aside, never overwritten.
- Commits follow conventional commits in lowercase; never add attribution footers.

## Commands

```powershell
pnpm install
pnpm tauri dev
pnpm lint
pnpm check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p xyra-core --test live_client -- --ignored --nocapture
pnpm tauri build
```

The smoke test needs League open; set `XYRA_LEAGUE_DIR` when it is not in `C:\Riot Games\League of Legends`, and
`XYRA_SMOKE_WRITE=1` to also import and remove a rune page.