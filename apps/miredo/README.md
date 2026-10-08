# MiReDo

MiReDo is a native Rust desktop application for browsing the local FFPM, Fihirana Fanampiny, Antema, and TSANTA song collections. It provides searchable lyrics and scores, favorites, and user-managed playlists.

## Run and test

From the workspace root:

```sh
cargo run --manifest-path apps/miredo/Cargo.toml
cargo test --manifest-path apps/miredo/Cargo.toml
```

Rust stable is required. The PDF reader uses the Poppler command-line tools `pdfinfo` and `pdftoppm`; those tools must be available on `PATH` on the target computer.

## Local data and privacy

- `data/` contains the four source JSON catalogs and their PDF scores. The app reads them directly and has no runtime network dependency.
- Favorites, playlists, reading positions, and preferences are saved in a local SQLite database in the operating system's per-user application-data directory.
- The upstream repository is used as a read-only source. Do not upload or push this local dataset into that repository.
- The source repository does not provide an explicit license in its metadata. Check rights before redistributing the imported songs or PDFs.

## Project layout

- `src/domain.rs` — normalized song, verse, collection, and playlist types.
- `src/data.rs` — source-catalog parsing, stable song IDs, and PDF matching.
- `src/search.rs` — local normalized search and filters.
- `src/storage.rs` — SQLite persistence for user-created state.
- `src/pdf.rs` — asynchronous PDF rendering through Poppler.
- `src/resources.rs` — centralized palette and localization loading.
- `src/ui.rs` — native desktop interface and application flow.
- `resources/i18n.json` — French, Malagasy, and English UI messages.
- `resources/palette.json` — light and dark semantic color tokens.
- `docs/miredo/` — the complete MiReDo design and requirements documentation.
