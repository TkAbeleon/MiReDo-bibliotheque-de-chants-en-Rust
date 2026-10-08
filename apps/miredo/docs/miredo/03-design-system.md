# MiReDo — Design system

## 1. Direction visuelle

MiReDo adopte une esthétique de **desktop productivity** :

- surfaces neutres ;
- accent limité ;
- typographie nette ;
- bordures fines ;
- ombres discrètes ;
- coins légèrement arrondis ;
- forte hiérarchie ;
- densité confortable ;
- commandes contextuelles.

L'inspiration « proche de Codex desktop » porte surtout sur l'organisation : sidebar compacte, espace principal dominant, contrôles sobres, vues centrées sur la tâche. Elle ne doit pas devenir une copie de produit.

## 2. Source unique des couleurs

Tous les tokens de couleur sont définis dans :

~~~text
resources/palette.json
~~~

Interdit dans les composants :

~~~text
"#315EFB"
rgb(...)
rgba(...)
~~~ 

Autorisé :

~~~text
color.background
color.surface
color.text_primary
color.border_subtle
color.accent
color.focus
~~~

## 3. Tokens de couleur

Catégories minimales :

### Neutres

~~~text
background
surface
surface_elevated
surface_hover
surface_selected
border_subtle
border_strong
text_primary
text_secondary
text_muted
~~~

### Sémantiques

~~~text
accent
accent_hover
accent_active
success
warning
danger
focus
~~~

### Lecteur

~~~text
viewer_background
viewer_paper
viewer_toolbar
viewer_selection
~~~

## 4. Palette

Les valeurs finales sont dans palette.json. La conception prévoit deux familles :

- Light ;
- Dark.

Le thème System sélectionne l'une des deux.

## 5. Typographie

| Niveau | Usage |
|---|---|
| Display | nom/branding |
| H1 | titre de page |
| H2 | titre de section |
| SongTitle | titre du chant |
| Body | paroles et contenu |
| Meta | auteur, catégorie, statut |
| Caption | aide et information secondaire |

Échelle indicative :

- titre application : 24–30 ;
- titre page : 22–28 ;
- titre chant : 20–26 ;
- corps : 14–16 ;
- métadonnées : 12–13.

Utiliser prioritairement une police système fiable sur chaque OS.

## 6. Espacement

Base 4 px :

~~~text
4   micro
8   compact
12  standard
16  contenu
20  séparation
24  section
32  majeur
40+ hero
~~~

## 7. Rayon

~~~text
sm   6
md   10
lg   14
xl   18
pill 999
~~~

Éviter les interfaces où chaque élément est une carte très arrondie.

## 8. Ombres

Ombre uniquement pour :

- menu ;
- popover ;
- dialog ;
- surface flottante.

Pas d'ombre forte sur chaque ligne de bibliothèque.

## 9. Composants

### Button

Variantes :

- Primary ;
- Secondary ;
- Ghost ;
- Destructive ;
- IconOnly.

### IconButton

Utilisé pour les actions fréquentes. Tooltip obligatoire sur les actions non évidentes.

### SearchField

Un composant partagé par :

- accueil ;
- bibliothèque ;
- lecteur.

### SongRow

~~~text
[N°]  Titre
      Auteur(s) · Catégorie                 [☆]
~~~

### Chip

Pour afficher un filtre actif.

### Dialog

Réservé aux actions nécessitant une décision :

- création ;
- renommage ;
- suppression.

### Toast

Pour les confirmations rapides.

## 10. États interactifs

Chaque composant doit définir :

~~~text
default
hover
pressed
focus-visible
selected
disabled
loading
error
~~~

Le focus clavier doit rester visible dans les deux thèmes.

## 11. Icônes

Pas d'emoji dans l'interface.

Utiliser des SVG cohérents et monochromes lorsque possible.

Taille de base : 16 px.

Tailles courantes : 18 / 20 / 24 px.

## 12. Densité

Objectif :

- 36–40 px pour actions compactes ;
- environ 44 px pour lignes de bibliothèque ;
- plus généreux pour la lecture du texte.

## 13. Accessibilité

Prévoir :

- navigation clavier ;
- focus-visible ;
- labels accessibles ;
- ordre logique des focus ;
- texte redimensionnable ;
- contraste suffisant ;
- option de réduction des animations.
