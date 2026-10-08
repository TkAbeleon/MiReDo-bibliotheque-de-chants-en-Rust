# MiReDo — Modèle de données

## 1. Entité centrale

~~~text
Song
├── id
├── number
├── category
├── title
├── authors[]
├── verses[]
├── pdf
└── source
~~~

Exemple d'identifiant stable :

~~~text
ff:001
ff:125
fanampiny:012
antema:03
tsanta:014
~~~

L'identifiant ne dépend jamais de la position dans le JSON.

## 2. Modèle Rust conceptuel

~~~text
Song {
    id: SongId,
    number: String,
    category: CategoryId,
    title: String,
    authors: Vec<AuthorRef>,
    verses: Vec<Verse>,
    pdf: Option<PdfRef>,
    source: SourceRef,
}
~~~

## 3. Mapping des JSON

| Source | Domaine |
|---|---|
| laharana | number |
| sokajy | category |
| lohateny | title |
| mpanoratra | authors |
| hira | verses |
| andininy | verse order |
| tononkira | text |
| fiverenany | is_refrain |

Le mapping est réalisé par un adaptateur de données.

## 4. Verse

~~~text
Verse
├── order
├── text
├── is_refrain
└── label?
~~~

Ne pas supposer que le refrain est toujours représenté par une valeur particulière de andininy.

## 5. Auteur

~~~text
Author
├── id
└── display_name
~~~

Les auteurs sont dédupliqués après normalisation Unicode et espaces.

Aucun auteur ne doit être inventé lorsque la source est vide.

## 6. Catégorie

Les catégories peuvent être :

- FFPM ;
- Fihirana Fanampiny ;
- Antema ;
- TSANTA.

L'identifiant technique est stable ; le libellé utilisateur est localisé.

## 7. Partition

~~~text
PdfRef
├── path
└── page_count?
~~~

Un PDF est une référence vers un fichier. L'application ne copie pas son contenu dans la base utilisateur.

États UI :

- disponible ;
- absent ;
- invalide.

## 8. Favoris

~~~text
Favorite
├── song_id
└── created_at
~~~

Le favori n'est jamais ajouté au JSON source.

## 9. Listes

~~~text
Playlist
├── id
├── name
├── created_at
├── updated_at
└── song_ids[]
~~~

Une liste ne contient que des références.

~~~text
Playlist "Répétition"
    ├── ff:012
    ├── ff:105
    └── tsanta:07
~~~

## 10. Préférences utilisateur

Prévoir :

- langue ;
- thème ;
- zoom initial ;
- layout simple/double ;
- dernière vue ;
- dernière liste ;
- dernières positions de lecture.

## 11. Validation au démarrage

Processus :

1. charger les quatre sources ;
2. normaliser ;
3. générer les IDs ;
4. détecter les collisions ;
5. vérifier les numéros ;
6. vérifier les catégories ;
7. résoudre les PDF ;
8. produire un diagnostic.

Une ligne invalide isolée ne doit pas empêcher toute la bibliothèque de se charger lorsque le reste est exploitable.
