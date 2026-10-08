# MiReDo — Documentation de conception

MiReDo est une application desktop multiplateforme écrite en Rust pour explorer, rechercher, consulter et organiser une bibliothèque de chants, avec deux modes de lecture interchangeables :

- **Partition** : affichage PDF de la partition.
- **Texte** : affichage des paroles/tononkira.

La direction visuelle privilégie une interface desktop moderne, sobre et ergonomique, inspirée des principes d'organisation et de densité des applications de productivité modernes, notamment l'esprit du Codex desktop, sans reproduire son interface ni son identité.

## Documents

| Document | Contenu |
|---|---|
| [00 — Vue d'ensemble](./00-vue-ensemble.md) | Vision, périmètre, exigences et principes |
| [01 — Architecture](./01-architecture.md) | Architecture Rust, modules, flux et responsabilités |
| [02 — Modèle de données](./02-modele-donnees.md) | Chant, auteurs, catégories, PDF, favoris et listes |
| [03 — Design system](./03-design-system.md) | Tokens, couleurs, typographie, espaces, composants et états |
| [04 — UX/UI](./04-ux-ui.md) | Layout, navigation, interactions, densité et responsive |
| [05 — Pages et navigation](./05-pages-navigation.md) | Accueil, bibliothèque, favoris, listes, lecteur, paramètres, aide et à propos |
| [06 — Lecteurs](./06-lecteurs.md) | Contrat commun PDF/Texte, zoom, pages et conservation du contexte |
| [07 — I18n et thèmes](./07-i18n-themes.md) | FR/MG/EN, fichier unique de traductions et palette unique |
| [08 — Persistance et recherche](./08-persistance-recherche.md) | SQLite, cache, recherche, filtres et préférences |
| [09 — Roadmap et qualité](./09-roadmap-qualite.md) | Découpage d'implémentation, tests, packaging et critères d'acceptation |

## Principes non négociables

1. Le **chant est l'entité centrale**. Les lecteurs PDF et texte sont deux représentations du même objet.
2. Le changement de lecteur conserve le contexte : même chant, même navigation et état de lecture pertinent.
3. **FR / MG / EN** sont gérés depuis **un seul fichier de traductions**.
4. Tous les tokens de couleur sont définis dans **un seul fichier de palette** ; aucune couleur UI ne doit être codée en dur dans les composants.
5. Les favoris et listes personnalisées stockent des **références de chants**, jamais des copies.
6. Les JSON sources restent séparés de la logique métier.
7. L'UI ne doit jamais manipuler directement la structure brute des JSON.
8. Le lecteur de partition maximise la surface utile : contrôles compacts, marges minimales et aucun panneau persistant masquant le document.
9. Les fonctions importantes restent accessibles au clavier.
10. MiReDo doit rester pleinement utilisable hors ligne une fois les données locales disponibles.
