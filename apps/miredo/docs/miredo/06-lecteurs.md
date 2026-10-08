# MiReDo — Conception des lecteurs

## 1. Principe

Un Song possède plusieurs représentations.

~~~text
Song
├── TextViewer
└── PdfViewer
~~~

Le viewer reçoit un SongId et utilise les repositories nécessaires via Application/Infrastructure.

## 2. Contrat conceptuel

~~~text
Viewer
├── open(song_id)
├── next()
├── previous()
├── zoom_in()
├── zoom_out()
└── toggle_fullscreen()
~~~

Le PDF ajoute la pagination ; le texte ajoute la position de défilement.

## 3. Conservation du contexte

~~~text
Song 125
   |
PDF
   |
Basculer
   |
Texte
~~~

Le système conserve :

- SongId ;
- résultat ou contexte de navigation ;
- mode ;
- état de lecture compatible.

Changer de viewer ne doit pas renvoyer à la page d'accueil.

## 4. PDF Viewer

Fonctions :

- rendu page par page ;
- zoom ;
- ajustement à la largeur ;
- ajustement à la page ;
- simple/double page ;
- page courante ;
- total de pages ;
- précédent/suivant ;
- plein écran.

## 5. Toolbar PDF

Proposition :

~~~text
[<] [>]      125 / 240     [-] 100% [+]
           [Simple][Double]      [⛶]
~~~

Les commandes secondaires sont regroupées dans un menu compact.

## 6. Double page

~~~text
+-------------------+-------------------+
|      page N       |      page N+1     |
+-------------------+-------------------+
~~~

Quand la fenêtre devient trop étroite, revenir automatiquement au mode simple.

## 7. Zoom

Modes possibles :

- Fit Width ;
- Fit Page ;
- niveau manuel.

Le zoom manuel est borné afin d'éviter des valeurs extrêmes.

## 8. Recherche dans le lecteur

La recherche de bibliothèque reste séparée du document.

Le champ peut être intégré à la toolbar.

Une recherche textuelle interne au PDF n'est pas une exigence MVP.

## 9. Performance PDF

Ne pas rendre tous les PDF à pleine résolution au démarrage.

Stratégie :

- rendu de la page visible ;
- préchargement voisin ;
- cache limité ;
- libération progressive.

Une opération de rendu ne doit pas bloquer durablement l'UI.

## 10. Text Viewer

Structure :

~~~text
TITRE
Auteur(s)
Catégorie

1.
Paroles

REFRAIN
Paroles

2.
Paroles
~~~

Principes :

- largeur de lecture contrôlée ;
- interligne confortable ;
- couples visuellement séparés ;
- refrain distinct ;
- aucune décoration inutile.

## 11. Actions communes

Les deux viewers fournissent :

- favori ;
- précédent/suivant ;
- ajout à une liste ;
- détails ;
- changement de mode.

## 12. Position de lecture

Mémorisation optionnelle :

~~~text
song_id + viewer -> position
~~~

Exemples :

~~~text
ff:125 + pdf  -> page 3
ff:125 + text -> scroll 0.48
~~~

## 13. Plein écran

Le plein écran masque la navigation générale mais conserve un accès discret aux commandes de lecture.

Escape doit toujours permettre la sortie.
