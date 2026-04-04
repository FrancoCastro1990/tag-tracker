# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build --release       # Build release binary
cargo test                  # Run all unit tests
cargo clippy                # Lint
cargo run -- <command>      # Run with args (e.g. cargo run -- tracker list)
```

Database is at `~/.local/share/tag-tracker/tag-tracker.db` (SQLite, WAL mode).

## Architecture

CLI time tracker for Hyprland/Waybar. Single binary, no daemon. Rust edition 2024.

**Data flow:** CLI command → clap parses → command handler → repo (SQL) → SQLite → signal waybar (`pkill -RTMIN+11`) for instant refresh.

### Module layout

- `cli/mod.rs` — clap derive structs (`Command`, `TrackerAction` enums)
- `cli/commands.rs` — command handlers + validation functions (color, rate, shortcut, date)
- `domain/tracker.rs` — `Tracker` struct, `TrackerState` enum (Created/Active/Paused), `TrackerType` enum (Freelance/Contract), `calculate_contract_rate()`
- `domain/session.rs` — `Session` struct (time intervals per tracker)
- `db/connection.rs` — `Database` struct, schema init, migrations via `PRAGMA user_version`
- `db/tracker_repo.rs` — tracker CRUD + `next_available_shortcut()` (1-9 allocation)
- `db/session_repo.rs` — session lifecycle + time/earnings calculations (`format_duration`, `format_clp`, `calculate_earnings`) + `today_seconds_for_date()` for date-specific queries
- `waybar/output.rs` — `WaybarOutput` JSON generation with pango markup (tracker color as foreground, aligned tooltip columns)
- `keybindings.rs` — generates `~/.config/hypr/tag-tracker-bindings.conf`, ensures `source` line in hyprland.conf, calls `hyprctl reload`
- `error.rs` — `AppError` enum (Database, Io, Validation, NotFound) via thiserror

### Key patterns

**DB migrations:** `connection.rs` uses `PRAGMA user_version` to track schema version. Migrations run in `Database::new()`. Current version: 3. Column detection (`SELECT col LIMIT 0`) handles fresh vs existing DBs. V3 added `tracker_type`, `salary`, `weekly_hours` columns.

**Only one active tracker:** `activate` pauses the current active tracker (if any) before activating the new one. State transitions: Created → Active ↔ Paused.

**Waybar integration:** Polls `tag-tracker waybar` every 5s + instant refresh via signal 11. Output is JSON with pango markup (tracker color as text foreground, CSS pill background). Click opens Walker picker via `tag-tracker menu`.

**Keybindings sync:** `keybindings::sync()` regenerates the bindings file from DB state. Called automatically after `tracker add/edit/delete`. Bindings use `SUPER ALT CTRL + 0-9`. The `ensure_hyprland_source()` function is idempotent.

**Stale session recovery:** `close_stale_sessions()` runs on every startup to close sessions left open from previous days (crash recovery).

**Tracker types:** Freelance trackers use a direct `--rate`. Contract trackers use `--contract --salary X --weekly-hours Y` and auto-calculate `hourly_rate = salary / (weekly_hours * 4.33)`. Both types store `hourly_rate` so downstream earnings logic is unchanged.

**CLP formatting:** Chilean peso format with dots as thousands separator (`$15.000` = 15 thousand).

## Testing

All tests use `Database::in_memory()` — no file I/O. Each test module has its own helper (`sample_tracker()`, `create_tracker()`, `setup_db_with_tracker()`) to set up test state. Tests cover repos, domain models, waybar output, and DB schema.
