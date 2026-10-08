# MiReDo

MiReDo is a native Rust desktop library for searching, reading, and organizing locally stored Malagasy hymn collections and PDF scores.

## Run & operate

- `cargo run --manifest-path apps/miredo/Cargo.toml` — start the native desktop app.
- `cargo test --manifest-path apps/miredo/Cargo.toml` — run the Rust tests.
- The VNC workflow named `MiReDo desktop` runs the app in the Replit desktop preview.
- Runtime PDF support requires Poppler's `pdfinfo` and `pdftoppm` commands on `PATH`.
- No API server, remote database, environment secret, or internet connection is required by MiReDo.

## Stack

- Rust stable and eframe/egui for the native desktop interface.
- SQLite with bundled SQLite library for local user state.
- JSON catalogs, localization, and palette are loaded from the app directory.
- Poppler utilities render the supplied PDF scores to images.

## Where things live

- `apps/miredo/src/domain.rs` — normalized app-level song and collection models.
- `apps/miredo/src/data.rs` — imports four JSON catalogs and matches PDFs to songs.
- `apps/miredo/src/search.rs` — accent-insensitive local search.
- `apps/miredo/src/storage.rs` — favorites, playlists, reading positions, and preferences.
- `apps/miredo/src/pdf.rs` — background PDF page rendering.
- `apps/miredo/src/ui.rs` — native screens and interaction flow.
- `apps/miredo/resources/i18n.json` — French, Malagasy, and English interface strings.
- `apps/miredo/resources/palette.json` — light/dark semantic colors.
- `apps/miredo/data/` — offline song catalogs and PDF files.
- `apps/miredo/docs/miredo/` — complete product documentation.

## Architecture decisions

- MiReDo is a Rust native desktop app, not a browser UI wrapped in a desktop shell.
- Source records are normalized before reaching the UI; user data stores stable song IDs rather than copied song records.
- Search and catalog access work locally; no runtime API or connection to the source repository is used.
- SQLite holds user preferences and collections, separate from the imported source catalogs.
- PDF rendering uses system Poppler tools so the source PDFs remain unchanged.

## Product

The app browses four hymn collections, searches by number/title/lyrics/author, opens text and PDF views for the same song, and saves favorites and named playlists across launches. It supports French, Malagasy, and English, plus light, dark, and system appearance modes.

## User preferences

- Keep the upstream `TkAbeleon/Fihirana-FFPM` repository unchanged. Use its data as local input in this separate MiReDo project; do not push the imported dataset back to it.

## Gotchas

- PDF viewing needs `pdfinfo` and `pdftoppm` installed and discoverable through `PATH`.
- The source repository's metadata does not declare a license; verify redistribution rights before sharing the imported corpus.

## Pointers

- See `apps/miredo/README.md` for setup and testing commands.
- See `apps/miredo/docs/miredo/` for the original product specification.
