# MiReDo

MiReDo is a native Rust desktop application for browsing the local FFPM, Fihirana Fanampiny, Antema, and TSANTA song collections. It provides searchable lyrics and scores, favorites, and user-managed playlists.

## Run and test

From the workspace root:

```sh
cargo run --manifest-path apps/miredo/Cargo.toml
cargo test --manifest-path apps/miredo/Cargo.toml
```

Rust stable is required. The PDF reader uses **PDFium** through the Rust crate `pdfium-render`. No `pdfinfo`/`pdftoppm` command-line tools are required.\n\n### PDFium runtime\n\n`pdfium-render` is the Rust wrapper; the native PDFium library must be available at runtime. MiReDo tries, in this order:\n\n1. `MIREDO_PDFIUM_PATH` if it is defined (a full library path or a directory containing the platform library).\n2. The directory containing the MiReDo executable.\n3. A PDFium library provided by the operating system.\n\nFor a portable distribution, package the matching PDFium library next to the MiReDo executable: `libpdfium.so` on Linux, `pdfium.dll` on Windows, or `libpdfium.dylib` on macOS. Prebuilt binaries are available from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases). Keep the native PDFium license notices with distributed binaries.

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
- `src/pdf.rs` — asynchronous PDFium rendering and page cache.
- `src/resources.rs` — centralized palette and localization loading.
- `src/ui.rs` — native desktop interface and application flow.
- `resources/i18n.json` — French, Malagasy, and English UI messages.
- `resources/palette.json` — light and dark semantic color tokens.
- `docs/miredo/` — the complete MiReDo design and requirements documentation.
