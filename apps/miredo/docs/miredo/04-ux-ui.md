# MiReDo — UX/UI et système d'interface

## 1. Fenêtre principale

Structure desktop :

~~~text
+------------------------------------------------------------------+
| barre supérieure / contrôles de fenêtre                          |
+----------------------+-------------------------------------------+
|                      |                                           |
|       Sidebar        |              Main content                  |
|      232–256 px      |                                           |
|                      |                                           |
+----------------------+-------------------------------------------+
~~~

Le contenu principal est toujours majoritaire.

## 2. Sidebar

Ordre :

~~~text
MiReDo
────────────────
Accueil
Bibliothèque
Favoris
Mes listes
────────────────
Aide
À propos
Paramètres
~~~

La sidebar est persistante sur écran desktop.

Mode compact :

~~~text
⌂
♫
★
☷
...
~~~

Les tooltips remplacent les labels.

## 3. Style général proche du desktop Codex

Principes retenus :

- navigation latérale stable ;
- surfaces discrètes ;
- zones sans bordures inutiles ;
- commandes regroupées ;
- contenu central large ;
- peu de couleurs ;
- peu d'ombres ;
- actions secondaires révélées au besoin ;
- états sélectionnés subtils.

MiReDo conserve sa propre identité musicale.

## 4. Accueil

~~~text
Bonjour

[ Rechercher un chant... ] [filtres]

Continuer
┌───────────────────────────────────────────────┐
│ 125  Titre du chant                           │
│      Auteur · FFPM                         ▶  │
└───────────────────────────────────────────────┘

Bibliothèque
...
~~~

L'accueil doit conduire immédiatement vers la recherche ou un chant récent.

## 5. Bibliothèque

~~~text
Bibliothèque                         [⌕ Rechercher...]

[Filtres actifs...]

N°     Titre                         Auteur         Catégorie
----------------------------------------------------------------
001    ...                           ...            FFPM
002    ...                           ...            FFPM
003    ...                           ...            TSANTA
~~~

Actions contextuelles apparaissent au hover ou à la sélection.

## 6. Recherche rapide

Raccourci :

- Ctrl+K sous Linux/Windows ;
- Cmd+K sous macOS.

Le champ peut devenir une palette de recherche compacte au-dessus du contenu, avec :

- requête ;
- catégorie ;
- auteur ;
- favori ;
- présence PDF.

Dans le lecteur PDF, la recherche est discrète et ne couvre pas la partition.

## 7. Filtres

Filtres MVP :

- catégorie ;
- auteur ;
- favori ;
- partition disponible ;
- liste.

Chaque filtre actif devient un chip supprimable individuellement.

## 8. Lecteur

Header :

~~~text
‹   précédent    125 · TITRE DU CHANT       ★   ...
~~~

Puis :

~~~text
[toolbar du viewer]
[contenu du viewer]
[navigation]
~~~

## 9. Vue PDF

La partition occupe le maximum d'espace.

Interdit :

- grand panneau de recherche au centre ;
- sidebar droite permanente ;
- contrôles géants ;
- marges décoratives excessives.

Autorisé :

- toolbar compacte ;
- popover temporaire ;
- panneau d'informations ouvert à la demande.

## 10. Vue texte

La page est centrée comme un document de lecture.

~~~text
                    TITRE DU CHANT
                    Auteur(s)

1.

Ligne de parole...
Ligne de parole...

REFRAIN

Ligne...
~~~

Le texte doit garder une largeur raisonnable pour la lisibilité.

## 11. Panneau contextuel

Pas de panneau droit permanent.

Lorsqu'il est nécessaire, il apparaît temporairement pour :

- détails ;
- ajout à une liste ;
- auteurs ;
- informations PDF.

Une action « fermer » et Escape le masquent.

## 12. Favoris

Le favori est disponible dans :

- bibliothèque ;
- résultats ;
- lecteur PDF ;
- lecteur texte ;
- listes.

Le symbole et les états doivent être identiques partout.

## 13. Listes

Création :

~~~text
Nouvelle liste
Nom
[_____________________]

[Annuler] [Créer]
~~~

Ajout :

~~~text
Ajouter à une liste

□ Répétition
□ Dimanche
□ Noël

+ Créer une liste
~~~

## 14. Paramètres

Organisation en sections :

~~~text
Paramètres
────────────
Apparence
Langue
Lecteur
Bibliothèque
Clavier
Données
~~~

Les options sont des contrôles simples et explicites.

## 15. Aide

Page structurée par sujets :

- Premiers pas ;
- recherche ;
- partition ;
- texte ;
- favoris ;
- listes ;
- paramètres ;
- raccourcis.

## 16. À propos

Contenu court :

- nom MiReDo ;
- version ;
- description ;
- données ;
- licence ;
- crédits ;
- composants logiciels.

## 17. Responsive desktop

Seuil pratique : environ 900 × 650 px.

Quand la largeur diminue :

1. sidebar compacte ;
2. métadonnées secondaires masquées ;
3. actions secondaires dans menu ;
4. colonnes de bibliothèque réduites ;
5. viewer prioritaire.

La structure reste la même ; seule la densité change.
