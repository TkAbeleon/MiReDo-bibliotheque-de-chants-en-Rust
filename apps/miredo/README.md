# MiReDo

MiReDo is a native Rust desktop application for browsing the local FFPM, Fihirana Fanampiny, Antema, and TSANTA song collections. It provides searchable lyrics and scores, favorites, and user-managed playlists.

## Run and test

From the workspace root:

```sh
cargo run --manifest-path apps/miredo/Cargo.toml
cargo test --manifest-path apps/miredo/Cargo.toml
```

Rust stable is required.

### PDF engine decision

The current codebase still uses `pdfium-render` for the working viewer, but the target architecture of MiReDo is a fully Rust PDF engine with no native system dependency.

We evaluated the published pure-Rust candidates that are actually available from crates.io:

- `hayro = "0.8.0"` — published crate, pure Rust PDF rasterizer, repository: https://github.com/LaurenzV/hayro
- `pdfboss-render = "2.13.1"` — published crate, pure Rust PDF rendering pipeline, repository: https://github.com/4thel00z/pdfboss

Both candidates comply with the architectural requirement of not depending on `PDFium`, `MuPDF`, `Poppler`, `pdftoppm`, `pdfinfo`, or other native libraries at runtime. The project should keep its abstraction layer independent from the renderer and migrate to one of these engines before shipping a production-ready, self-contained package.

### Target architecture for the renderer

MiReDo should expose a pure Rust renderer abstraction of the form:

```rust
trait PdfEngine {
    fn open(&mut self, path: &std::path::Path) -> anyhow::Result<()>;
    fn page_count(&self) -> anyhow::Result<usize>;
    fn page_size(&self, page: usize) -> anyhow::Result<[u32; 2]>;
    fn render_page(
        &mut self,
        page: usize,
        width: u32,
        height: u32,
    ) -> anyhow::Result<crate::pdf::RenderedPage>;
    fn extract_text(&self, page: usize) -> anyhow::Result<String>;
}
```

Then the egui layer should depend on the abstraction only, never directly on the implementation details of Hayro or pdfboss.

### Recommended choice

The preferred migration target is Hayro because it is a dedicated PDF rasterizer library with a stable published version and an explicit rendering API. pdfboss-render is the second candidate to benchmark on the real sample PDFs from `apps/miredo/data/`, because it is also pure Rust and may offer a different trade-off in text extraction and rendering features.

The final engine choice must be validated on the real documents in this repository, not on advertisements alone.

### PDFium compatibility note

This note remains for the current transitional implementation only:

1. `MIREDO_PDFIUM_PATH` if it is defined.
2. The directory containing the MiReDo executable.
3. Local bundle folders such as `pdfium/`, `lib/`, or `vendor/pdfium/` next to the executable.
4. A PDFium library provided by the operating system.

This is a temporary fallback while the project migrates to a pure Rust PDF engine.

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
