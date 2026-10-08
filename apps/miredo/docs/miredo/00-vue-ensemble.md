# MiReDo — Vue d'ensemble

## 1. Vision

MiReDo est une bibliothèque numérique de chants pensée pour la recherche rapide, la lecture de paroles et la consultation de partitions.

L'application n'est pas un simple lecteur PDF. Le PDF est une représentation d'un **chant structuré**.

~~~text
                       CHANT
                         |
        +----------------+----------------+
        |                |                |
      Texte           Partition       Métadonnées
        |                |                |
        +----------------+----------------+
                         |
               +---------+---------+
               |                   |
           Favori            Listes utilisateur
~~~

## 2. Sources

Le premier corpus provient des collections du dépôt Fihirana-FFPM :

- Fihirana FFPM
- Fihirana Fanampiny
- Antema
- TSANTA

Les JSON historiques utilisent notamment :

- laharana : numéro ;
- sokajy : catégorie ;
- lohateny : titre ;
- mpanoratra : auteurs ;
- hira : parties du chant ;
- andininy : numéro du couplet ;
- tononkira : texte ;
- fiverenany : indication de refrain.

MiReDo introduit une couche de normalisation afin de ne pas exposer ces noms de champs au reste du produit.

## 3. Fonctionnalités

### Bibliothèque

- recherche par numéro ;
- recherche par titre ;
- recherche dans les paroles ;
- recherche par auteur ;
- filtres par catégorie ;
- indication de disponibilité de la partition ;
- ouverture directe d'un chant.

### Lecture

- lecteur PDF ;
- lecteur texte ;
- passage PDF ↔ Texte ;
- chant précédent/suivant ;
- zoom ;
- page simple ;
- double page ;
- plein écran ;
- navigation clavier ;
- conservation du contexte.

### Organisation

- ajouter/retirer un favori ;
- liste des favoris ;
- créer, renommer et supprimer une liste ;
- ajouter/retirer un chant d'une liste ;
- réordonner les chants d'une liste dans une évolution ultérieure.

### Application

- paramètres ;
- thème clair/sombre/système ;
- langues FR/MG/EN ;
- aide ;
- raccourcis clavier ;
- à propos ;
- version et crédits.

## 4. Utilisateurs

### Recherche rapide

Trouver un chant à partir d'un numéro, d'un titre partiel, d'un auteur ou d'une phrase.

### Musicien

Ouvrir la partition, garder le contenu dominant et utiliser la navigation clavier.

### Responsable de répétition

Préparer des listes personnelles de chants.

### Utilisateur occasionnel

Comprendre l'application sans connaître sa structure technique.

## 5. Principes UX

### Contenu avant chrome

Le contenu musical est toujours prioritaire sur les commandes.

### Révélation progressive

Les fonctions secondaires apparaissent au besoin.

### Contexte conservé

Une action ne doit pas perdre le chant courant ou la liste de recherche.

### Prévisible

Une même action conserve son apparence et sa position.

### Réversible

Les actions destructives sont confirmées ou annulables.

## 6. Cible multiplateforme

- Linux ;
- Windows ;
- macOS lorsque le pipeline de compilation et de packaging est disponible.

Les dépendances au système sont encapsulées dans Platform.

## 7. Hors périmètre MVP

MiReDo n'est pas :

- un logiciel de composition ;
- un éditeur de partition ;
- un moteur d'OCR de partitions ;
- une plateforme de partage en ligne.

Le MVP consulte les données locales.
