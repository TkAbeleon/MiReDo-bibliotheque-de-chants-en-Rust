# MiReDo — Choix technologiques et règles d'implémentation

## 1. Objectif

Le choix technologique doit rester cohérent avec quatre contraintes :

- application desktop native ;
- Rust comme langage principal ;
- Linux + Windows, avec macOS visé ;
- interface moderne sans transformer le projet en application web emballée.

## 2. Couche UI

Le toolkit UI sera choisi à l'issue d'un spike technique court, selon ces critères :

| Critère | Exigence |
|---|---|
| Rust natif | obligatoire |
| Linux | obligatoire |
| Windows | obligatoire |
| macOS | souhaitable |
| thèmes | clair/sombre/système |
| accessibilité clavier | obligatoire |
| rendu texte | excellent |
| SVG/icônes | nécessaire |
| scrolling | nécessaire |
| PDF intégrable | nécessaire ou intégrable par composant |
| fenêtre native | nécessaire |

Le framework ne doit pas imposer une architecture qui mélange les données et l'interface.

## 3. Préférence d'architecture UI

Le projet doit privilégier une architecture déclarative/composants avec :

~~~text
App State
   ↓
Commands / Events
   ↓
Pages
   ↓
Reusable Components
   ↓
Theme + I18n
~~~

Le renderer PDF est isolé du reste du système.

## 4. Persistance

SQLite est la cible de persistance locale pour les données utilisateur.

La bibliothèque de chants est reconstruisible à partir des sources JSON.

## 5. PDF

Le moteur PDF doit permettre au minimum :

- rendu rasterisé ou vectoriel de bonne qualité ;
- navigation de pages ;
- zoom ;
- rotation si le fichier l'exige ;
- cache de pages ;
- récupération du nombre de pages.

L'abstraction `PdfRenderer` évite de lier le domaine à une bibliothèque PDF précise.

## 6. Recherche

Le MVP doit privilégier une recherche locale, rapide et prévisible.

Une abstraction `SearchEngine` permet d'évoluer de :

~~~text
index en mémoire
        ↓
cache local
        ↓
FTS SQLite éventuel
~~~

sans modifier l'interface.

## 7. Ressources

Les ressources stables de l'application :

~~~text
resources/
├── i18n.json
└── palette.json
~~~

### i18n

Une seule source pour :

- français ;
- malagasy ;
- anglais ;
- labels ;
- boutons ;
- tooltips ;
- messages ;
- aide ;
- à propos.

### Palette

Une seule source pour :

- couleurs light ;
- couleurs dark ;
- couleurs sémantiques ;
- couleurs spécifiques au viewer ;
- états focus/sélection/erreur.

## 8. Configuration utilisateur

Les préférences locales ne doivent pas être écrites dans le dépôt.

Exemples :

~~~text
locale
theme
sidebar_width
pdf_zoom
pdf_layout
reduce_motion
last_song
last_viewer
~~~

## 9. Style Codex-like : ce qui est repris

MiReDo peut reprendre les principes suivants :

### Navigation de travail

Sidebar persistante et compacte.

### Surface principale

Une zone de contenu dominante, avec une hiérarchie très claire.

### Commandes contextuelles

Les commandes secondaires apparaissent lorsque le contexte le justifie.

### Densité intelligente

Beaucoup d'informations utiles, mais sans grille visuelle lourde.

### État explicite

Sélection, focus, chargement et erreurs sont toujours perceptibles.

## 10. Ce qui n'est pas repris

MiReDo ne copie pas :

- le logo ;
- les icônes ;
- les textes ;
- les composants propriétaires ;
- les couleurs exactes ;
- les écrans ;
- les interactions métier d'un autre produit.

Le but est de reprendre des principes UX généraux, pas de cloner une application.

## 11. Convention de code

Nommer selon le domaine :

~~~text
Song
Playlist
ViewerState
SearchQuery
ThemeMode
Locale
~~~

Éviter des noms UI vagues comme :

~~~text
Manager
Helper
DataThing
Utils2
~~~

Les fonctions doivent exprimer leur intention.

## 12. Convention des fichiers

Les modules doivent rester petits et spécialisés.

Un fichier ne doit pas devenir un « fourre-tout ».

Les composants partagés sont placés dans :

~~~text
src/ui/components/
~~~

Les pages dans :

~~~text
src/ui/pages/
~~~

Les services dans :

~~~text
src/application/
~~~

## 13. Règle de dépendance

Direction obligatoire :

~~~text
UI
 ↓
Application
 ↓
Domain
 ↑
Infrastructure
~~~

La technologie PDF, SQLite ou UI ne doit pas pénétrer le modèle Song.

## 14. Sécurité et données

MiReDo est principalement hors ligne.

Ne pas collecter de télémétrie par défaut.

Ne pas exiger de compte utilisateur pour le MVP.

Les données personnelles éventuelles se limitent aux préférences et organisations locales créées par l'utilisateur.

## 15. Décision avant implémentation

Avant de figer le toolkit UI et le moteur PDF, réaliser un prototype minimal contenant :

1. sidebar ;
2. thème clair/sombre ;
3. i18n FR/MG/EN ;
4. SongRow ;
5. affichage d'un PDF ;
6. scrolling texte ;
7. navigation précédent/suivant ;
8. redimensionnement de fenêtre.

Le choix final doit être fait sur une application réelle, pas uniquement sur une comparaison théorique.
