# MiReDo — Roadmap, qualité et critères d'acceptation

## 1. Phase 0 — Fondation

- initialiser le projet Rust ;
- définir les modules ;
- charger i18n.json ;
- charger palette.json ;
- installer logs et erreurs ;
- afficher le shell UI.

### Validation

L'application démarre avec un thème et une langue cohérents.

## 2. Phase 1 — Données

- parser FFPM ;
- parser Fihirana Fanampiny ;
- parser Antema ;
- parser TSANTA ;
- normaliser vers Song ;
- valider les IDs ;
- détecter les partitions.

### Validation

Tous les chants exploitables apparaissent dans une collection unique.

## 3. Phase 2 — Bibliothèque

- accueil ;
- liste des chants ;
- recherche ;
- filtres ;
- vue par auteur ;
- indication PDF.

### Validation

Un chant est retrouvable par numéro, titre, auteur et paroles.

## 4. Phase 3 — Text Viewer

- titre ;
- auteur ;
- couplets ;
- refrains ;
- navigation ;
- favori ;
- ajout à une liste.

## 5. Phase 4 — PDF Viewer

- rendu ;
- zoom ;
- simple/double page ;
- page courante ;
- navigation ;
- plein écran ;
- bascule PDF/Texte.

### Critère critique

La partition doit rester le contenu visuellement dominant.

## 6. Phase 5 — Organisation

- favoris ;
- listes ;
- création ;
- renommage ;
- suppression ;
- ajout/retrait ;
- persistance.

## 7. Phase 6 — Paramètres / Aide / À propos

- FR/MG/EN ;
- Light/Dark/System ;
- paramètres lecteur ;
- paramètres bibliothèque ;
- aide ;
- raccourcis ;
- à propos.

## 8. Phase 7 — Qualité

Tester :

- petite fenêtre ;
- grande fenêtre ;
- thème clair ;
- thème sombre ;
- clavier ;
- souris ;
- PDF long ;
- PDF absent ;
- données incomplètes ;
- redémarrage après modifications utilisateur.

## 9. Tests unitaires

- parser ;
- normalisation ;
- SongId ;
- recherche ;
- filtres ;
- listes ;
- favoris ;
- préférence.

## 10. Tests d'intégration

- chargement complet ;
- persistance SQLite ;
- rechargement ;
- ouverture ;
- changement de viewer ;
- conservation du contexte.

## 11. Performance

Objectifs :

- recherche locale quasi instantanée ;
- aucun rendu PDF global au démarrage ;
- mémoire maîtrisée ;
- UI non bloquée pendant les tâches coûteuses.

## 12. Diagnostic de données

Au démarrage, permettre de produire :

~~~text
Songs loaded: ...
Songs with authors: ...
Songs with lyrics: ...
Songs with PDF: ...
Duplicate IDs: ...
Invalid records: ...
~~~

## 13. Packaging

Cibles :

- Linux : AppImage ou paquet natif ;
- Windows : installateur ;
- macOS : app bundle.

Les ressources et données doivent avoir une stratégie de déploiement documentée.

## 14. Critères d'acceptation du MVP

Le MVP est accepté lorsque :

- les quatre sources sont chargées ;
- recherche numéro/titre/auteur/paroles fonctionne ;
- un chant peut être ouvert ;
- PDF et Texte sont deux vues du même chant ;
- le contexte est conservé lors du changement de viewer ;
- favoris persistent ;
- listes persistent ;
- FR/MG/EN utilisent une source de traduction unique ;
- Light/Dark/System utilisent une palette unique ;
- Paramètres, Aide et À propos existent ;
- l'interface reste utilisable lorsque la fenêtre est redimensionnée ;
- le produit fonctionne hors ligne.

## 15. Règle avant fusion

Toute nouvelle fonctionnalité doit répondre à :

1. où se situe-t-elle architecturalement ?
2. tous ses textes sont-ils dans i18n.json ?
3. toutes ses couleurs viennent-elles de palette.json ?
4. respecte-t-elle les deux viewers ?
5. dispose-t-elle d'au moins un test pertinent ?

Si une de ces réponses est non, l'implémentation doit être revue.
