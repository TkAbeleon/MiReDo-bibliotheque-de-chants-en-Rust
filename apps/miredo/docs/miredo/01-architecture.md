# MiReDo — Architecture logicielle

## 1. Architecture en couches

~~~text
+------------------------------------------------------+
|                         UI                           |
| pages / composants / navigation / états visuels    |
+-------------------------------+----------------------+
                                |
+-------------------------------v----------------------+
|                      Application                    |
| Search / OpenSong / Viewer / Favorite / Playlist   |
+-------------------------------+----------------------+
                                |
+-------------------------------v----------------------+
|                         Domain                      |
| Song / Author / Collection / Playlist / Preference |
+-------------------------------+----------------------+
                                |
+-------------------------------v----------------------+
|                   Infrastructure                    |
| JSON / SQLite / PDF / filesystem / platform       |
+------------------------------------------------------+
~~~

La dépendance va vers le cœur métier. Les couches basses ne doivent pas connaître les pages UI.

## 2. Arborescence cible

~~~text
miredo/
├── Cargo.toml
├── data/
│   ├── 01_fihirana_ffpm.json
│   ├── 02_fihirana_fanampiny.json
│   ├── 03_antema.json
│   ├── 04_tsanta.json
│   └── pdf/
├── resources/
│   ├── i18n.json
│   └── palette.json
├── docs/
│   └── miredo/
├── src/
│   ├── main.rs
│   ├── app/
│   ├── domain/
│   ├── application/
│   ├── infrastructure/
│   ├── search/
│   ├── viewer/
│   ├── ui/
│   ├── i18n/
│   ├── theme/
│   ├── platform/
│   └── error.rs
└── tests/
~~~

## 3. Domain

Le domaine contient des modèles purs :

~~~text
Song
Author
Category
Verse
PdfRef
Playlist
Favorite
Preference
~~~

Aucun accès direct au système de fichiers ou à la base.

## 4. Application

Les cas d'usage deviennent des fonctions/services testables :

~~~text
search_songs(query, filters)
get_song(song_id)
toggle_favorite(song_id)
create_playlist(name)
rename_playlist(id, name)
delete_playlist(id)
add_song_to_playlist(playlist_id, song_id)
remove_song_from_playlist(playlist_id, song_id)
open_song(song_id)
set_viewer_mode(mode)
set_locale(locale)
set_theme(theme)
~~~

## 5. Infrastructure

Responsabilités :

- parser les JSON ;
- mapper les données vers Domain ;
- charger les PDF ;
- effectuer le rendu ;
- persister l'état utilisateur ;
- résoudre les chemins OS ;
- écrire les logs.

## 6. UI

L'UI affiche un état applicatif et envoie des commandes.

Elle ne doit pas :

- parser un JSON ;
- écrire directement dans SQLite ;
- construire elle-même un chemin de PDF ;
- contenir les traductions ;
- contenir des couleurs hexadécimales.

## 7. AppState

~~~text
AppState
├── current_page
├── current_song_id
├── current_viewer
├── current_results
├── search_query
├── active_filters
├── selected_playlist
├── locale
├── theme
└── sidebar_state
~~~

## 8. ViewerState

~~~text
ViewerState
├── song_id
├── mode
├── pdf_page
├── zoom
├── page_layout
├── fullscreen
└── reading_position
~~~

Le ViewerState ne remplace pas le Song. Il indique comment le Song est actuellement présenté.

## 9. Commandes et événements

~~~text
OpenSong
NextSong
PreviousSong
ToggleFavorite
CreatePlaylist
RenamePlaylist
DeletePlaylist
AddToPlaylist
RemoveFromPlaylist
SwitchViewer
SetZoom
SetPageLayout
ToggleFullscreen
SetTheme
SetLocale
~~~

Les événements sont utiles pour éviter les dépendances entre composants.

## 10. Gestion des erreurs

Erreurs typées :

- DataLoadError ;
- SongNotFound ;
- PdfNotFound ;
- PdfRenderError ;
- PersistenceError ;
- TranslationError ;
- InvalidPlaylistName.

L'UI traduit une erreur technique en message utilisateur compréhensible.

## 11. Moteur PDF et architecture de rendu

MiReDo doit rester compatible avec une architecture de lecture PDF asynchrone, sans dépendance à une bibliothèque native système. Le moteur PDF est un composant d'infrastructure, pas une couche UI.

Le candidat principal est Hayro (`hayro = "0.8.0"`), avec `pdfboss-render = "2.13.1"` comme deuxième benchmark.

### Bloc de conception

~~~rust
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
~~~

L'implémentation derrière cette interface doit respecter les contraintes suivantes :

- 100 % Rust, sans `pdfium-render`, `MuPDF`, `Poppler`, `pdftoppm`, `pdfinfo` ni autre binding C/C++ ;
- ouverture du document une seule fois ;
- document conservé en mémoire ;
- pages rendues hors thread UI ;
- cache mémoire local pour la page courante / précédente / suivante ;
- rendu adaptatif selon taille du viewport et zoom ;
- interface de lecture indépendante de l'implémentation technique ;
- benchmark sur les PDFs réels dans `apps/miredo/data/` avant choix final.

### Critères de sélection

Le moteur retenu doit être choisi après validation réelle sur les critères suivants :

1. compilation Rust stable ;
2. ouverture des PDFs du dépôt ;
3. rendu des partitions ;
4. rendu des lignes musicales ;
5. texte et paroles ;
6. proportions ;
7. annotations ;
8. vitesse ;
9. mémoire ;
10. compatibilité Linux ;
11. possibilité Windows / macOS ;
12. licence compatible.

Le moteur ne doit pas être décisionné uniquement sur la réputation, mais sur la mesure sur les documents MiReDo.

## 12. Observabilité

Niveaux recommandés :

- INFO : démarrage, chargement, ouverture de chant ;
- DEBUG : indexation, cache, rendu ;
- WARN : auteur absent, PDF absent ;
- ERROR : corruption ou échec de persistance.

Pas de logs inutilement sensibles.
