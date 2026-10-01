# AlgoCompiler

[![CI](https://github.com/BetterIUTInfoAix/AlgoCompiler/actions/workflows/ci.yml/badge.svg)](https://github.com/BetterIUTInfoAix/AlgoCompiler/actions/workflows/ci.yml)

Compilateur et interpréteur écrits en Rust pour le langage algorithmique
« papier » de l'IUT d'Aix (projet BetterAlgoPapier). Il **exécute** un
programme `.algo` et le **compile vers Python** — en ligne de commande
comme dans le navigateur (WebAssembly, en local).

```text
declarer note : entier <- 12;
si (note >= 10) afficher("reçu");
sinon afficher("raté");
fsi
```

## Essayer

**Dans le navigateur** (rien à installer) : [le Playground](https://betteriutinfoaix.github.io/AlgoCompiler/)
— choisissez un exemple, remplissez les entrées, cliquez **Exécuter** (ou
**Compiler** pour voir le Python).

**En local** :

```bash
git clone https://github.com/BetterIUTInfoAix/AlgoCompiler.git
cd AlgoCompiler/AlgoCompiler
cargo run -- exemples/hello.algo   # affiche le Python généré
```

## Fonctionnalités

- Variables (6 types, `tableau_de`, `constante`), affectations, arithmétique,
  comparaisons (`vaut`, `ne_vaut_pas`…), logique (`et`, `ou`, `non`…) ;
- conditions (`si` court sans `fsi` + forme longue, `choix_sur`), 5 boucles
  (`pour`, `tant_que`, `jusqua`, `repeter`, `boucle` + `sortie`/`continue`) ;
- sous-programmes (`fonction`/`procedure` avec `in`/`out`/`in_out`,
  `renvoie`, `algorithme debut fin`), `saisir`, intégrées
  (`taille`, `modulo`, `rand`) ;
- erreurs localisées en français (ligne, colonne, extrait, aide), codes
  stables `E001`–`E301` ;
- zéro dépendance externe (Rust édition 2024).

## Documentation

Le détail est dans le **[wiki](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki)** :

- [Installation](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Installation), [CLI](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Utilisation-CLI), [Playground](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Playground) ;
- [Langage supporté](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Langage-supporte), [Exécution](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Execution), [Erreurs](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Erreurs), [Exemples](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Exemples) ;
- [Architecture](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Architecture), [Contribuer](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Contribuer).

Cours du langage (pédagogie L1) : [Wiki-BetterAlgoPapier](https://betteriutinfoaix.github.io/Wiki-BetterAlgoPapier/).

## Développer

```bash
cd AlgoCompiler
cargo test                          # 122 tests
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
cd ../playground && ./build.sh      # module WASM (pkg/)
python3 -m http.server 8000        # depuis playground/
```

Voir [[Contribuer](https://github.com/BetterIUTInfoAix/AlgoCompiler/wiki/Contribuer)] pour le workflow et la feuille de route (nouveaux générateurs C/C++/JS, vérification des types…).
