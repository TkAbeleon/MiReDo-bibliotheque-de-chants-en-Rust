# MiReDo — Internationalisation et thèmes

## 1. Langues officielles

MiReDo est trilingue :

- Français — fr ;
- Malagasy — mg ;
- English — en.

## 2. Un seul fichier de traduction

Toutes les chaînes UI résident dans :

~~~text
resources/i18n.json
~~~

Structure :

~~~text
i18n.json
├── fr
├── mg
└── en
~~~

Il n'y a pas de séparation en trois fichiers dans le MVP.

## 3. Clés sémantiques

Exemples :

~~~text
nav.home
nav.library
nav.favorites
nav.playlists
nav.settings
nav.help
nav.about

search.placeholder

viewer.pdf
viewer.text
viewer.next
viewer.previous

settings.language
settings.theme
~~~

Les composants ne connaissent pas la phrase traduite.

## 4. Interdiction de chaînes UI en dur

Mauvais :

~~~text
"Paramètres"
"Settings"
"Fikirana"
~~~

Bon :

~~~text
t("nav.settings")
~~~

## 5. Aide et textes longs

Les contenus de :

- Aide ;
- À propos ;
- dialogues ;
- messages système ;
- tooltips ;

doivent eux aussi venir de i18n.json.

## 6. Fallback

Résolution :

~~~text
préférence utilisateur
        ↓
langue système
        ↓
français
~~~

Une clé manquante doit avoir un fallback propre pour éviter une interface cassée.

## 7. Source unique des couleurs

Toutes les couleurs sont définies ici :

~~~text
resources/palette.json
~~~

## 8. Schéma de palette

~~~text
palette.json
├── light
│   ├── background
│   ├── surface
│   ├── text_primary
│   ├── text_secondary
│   ├── border_subtle
│   ├── accent
│   ├── success
│   ├── warning
│   └── danger
└── dark
    ├── background
    ├── surface
    ├── text_primary
    ├── text_secondary
    ├── border_subtle
    ├── accent
    ├── success
    ├── warning
    └── danger
~~~

## 9. Rôle des tokens

Les composants utilisent des rôles sémantiques :

~~~text
text_primary
surface
surface_hover
border_subtle
accent
danger
focus
~~~

Pas des noms comme blue_500 ou gray_700.

## 10. Thèmes

Modes :

- System ;
- Light ;
- Dark.

Les composants ne changent pas de structure lorsque le thème change.

## 11. Accessibilité

Prévoir :

- contraste suffisant ;
- focus visible ;
- texte lisible ;
- indication non exclusivement basée sur la couleur ;
- réduction des animations.

## 12. Animation

Animations de courte durée pour :

- ouverture de menu ;
- feedback ;
- changements légers.

Aucune animation décorative permanente.

Le réglage « réduire les animations » désactive les mouvements non essentiels.
