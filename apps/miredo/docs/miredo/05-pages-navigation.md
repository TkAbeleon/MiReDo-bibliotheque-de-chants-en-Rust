# MiReDo — Pages et navigation

## 1. Carte fonctionnelle

~~~text
Accueil
├── Bibliothèque
│   └── Chant
│       ├── Partition
│       └── Texte
├── Favoris
├── Mes listes
│   └── Liste
├── Paramètres
├── Aide
└── À propos
~~~

## 2. Accueil

Fonctions :

- recherche rapide ;
- accès bibliothèque ;
- reprise du dernier chant ;
- accès favoris ;
- aperçu des collections.

## 3. Bibliothèque

Vue principale du catalogue.

Chaque SongRow propose :

- ouverture ;
- favori ;
- ajout à une liste ;
- détails.

## 4. Favoris

La page est une bibliothèque filtrée sur l'état favori.

Aucun modèle de donnée parallèle n'est nécessaire.

## 5. Mes listes

Vue :

~~~text
Mes listes

Répétition         18 chants
Dimanche            7 chants
Noël                12 chants
~~~

Une liste ouverte conserve son identité jusqu'à la fermeture.

## 6. Lecteur

Le lecteur possède un contexte partagé :

~~~text
SongHeader
ViewerToolbar
ViewerContent
ViewerNavigation
~~~

Le contenu change selon le mode.

## 7. Paramètres

### Apparence

- Système ;
- Clair ;
- Sombre.

### Langue

- Français ;
- Malagasy ;
- English.

### Lecteur

- page simple ;
- double page ;
- zoom initial ;
- position ;
- comportement des commandes.

### Bibliothèque

- dossier de données ;
- dossier PDF ;
- recharger.

### Clavier

Liste des raccourcis.

## 8. Aide

La page d'aide est entièrement trilingue et couvre :

1. démarrage ;
2. recherche ;
3. lecture ;
4. PDF ;
5. texte ;
6. favoris ;
7. listes ;
8. paramètres ;
9. clavier.

## 9. À propos

~~~text
MiReDo
Bibliothèque et lecteur de chants

Version x.y.z

Données
Licence
Crédits
Bibliothèques utilisées
~~~

## 10. Navigation clavier

| Action | Linux/Windows | macOS |
|---|---|---|
| Recherche | Ctrl+K | Cmd+K |
| Fermer | Esc | Esc |
| Chant suivant | N / PageDown | N / PageDown |
| Chant précédent | P / PageUp | P / PageUp |
| Favori | F | F |
| PDF/Texte | V | V |
| Plein écran | F11 | Ctrl+Cmd+F |
| Zoom + | + | + |
| Zoom - | - | - |

Les raccourcis sont des defaults. Une personnalisation peut venir après le MVP.
