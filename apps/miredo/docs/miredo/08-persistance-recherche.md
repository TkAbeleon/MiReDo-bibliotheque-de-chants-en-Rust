# MiReDo — Persistance, recherche et bibliothèque locale

## 1. Séparer source et état utilisateur

Les JSON sont les données de référence.

SQLite contient l'état utilisateur.

~~~text
JSON
└── chants / paroles / auteurs / catégories

SQLite
├── favoris
├── listes
├── membres de listes
├── préférences
└── positions
~~~

## 2. Schéma SQLite minimal

~~~sql
favorites (
    song_id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL
);

playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

playlist_songs (
    playlist_id TEXT NOT NULL,
    song_id TEXT NOT NULL,
    position INTEGER,
    PRIMARY KEY (playlist_id, song_id)
);

preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

reading_positions (
    song_id TEXT NOT NULL,
    viewer TEXT NOT NULL,
    position REAL NOT NULL,
    PRIMARY KEY (song_id, viewer)
);
~~~

## 3. Cache

Un cache normalisé peut être généré à partir des JSON pour accélérer :

- recherche ;
- filtres ;
- comptages.

Ce cache doit être reconstruisible.

## 4. Recherche

Champs :

~~~text
number
title
authors
lyrics
category
~~~

## 5. Normalisation

Avant indexation :

1. normaliser Unicode ;
2. passer en casse neutre ;
3. normaliser espaces ;
4. traiter les apostrophes ;
5. neutraliser la ponctuation non pertinente ;
6. produire une forme de comparaison tolérante.

Le texte original reste intact pour l'affichage.

## 6. Priorité des résultats

Ordre logique :

~~~text
numéro exact
    >
titre exact
    >
titre préfixe
    >
auteur exact
    >
titre partiel
    >
paroles
~~~

La stabilité du tri est importante.

## 7. Filtres

Filtres composables :

- catégorie ;
- auteur ;
- favori ;
- présence PDF ;
- liste.

## 8. Rechargement

~~~text
charger JSON
   ↓
valider
   ↓
normaliser
   ↓
reconstruire index
   ↓
préserver favoris/listes
~~~

Les listes utilisent les SongId et ne dépendent donc pas de l'ordre des fichiers.

## 9. Hors ligne

Une fois les données locales présentes, les opérations essentielles n'exigent aucun réseau.

## 10. Emplacements système

Les chemins sont déterminés par Platform et non par des chemins écrits en dur.

Exemples à éviter :

~~~text
/home/user/...
C:\Users\...
~~~

## 11. Données volumineuses

Ne pas charger inutilement toutes les pages PDF.

Les paroles peuvent être gardées en mémoire car le corpus reste beaucoup plus léger qu'un ensemble de documents PDF rendus.
