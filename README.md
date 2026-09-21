# AlgoCompiler

AlgoCompiler est un prototype de compilateur écrit en Rust pour le langage algorithmique utilisé dans le projet BetterAlgoPapier. Il transforme une instruction algorithmique en code Python exécutable.

Le projet est encore en développement. Pour le moment, il prend en charge l'instruction `afficher` avec une chaîne de caractères.

## Exemple

Code algorithmique :

```text
afficher("Bonjour, monde !");
```

Code Python généré :

```python
print("Bonjour, monde !")
```

L'exemple actuellement exécuté par le binaire est défini directement dans `src/main.rs`.

## Prérequis

- [Rust et Cargo](https://www.rust-lang.org/tools/install)

Le projet utilise l'édition Rust 2024 et ne possède actuellement aucune dépendance externe.

## Installation et utilisation

Depuis le dossier `AlgoCompiler/` :

```bash
cargo run
```

Cette commande compile le projet puis affiche le code algorithmique utilisé et le code Python généré.

Pour exécuter les tests :

```bash
cargo test
```

Pour vérifier le projet sans lancer le binaire :

```bash
cargo check
```

## Fonctionnement

Le compilateur est organisé en plusieurs étapes :

1. Le lexer (`src/lexer/`) transforme le texte en tokens.
2. Le parser (`src/parser/`) transforme les tokens en arbre syntaxique abstrait.
3. Le générateur (`src/codegen/`) produit le code Python à partir de cet arbre.

Les types principaux sont :

- `Token` : représente les éléments reconnus par le lexer ;
- `Program` : représente un programme complet ;
- `Statement::Afficher` : représente une instruction d'affichage.

## Syntaxe supportée

Une instruction d'affichage respecte la forme suivante :

```text
afficher("texte");
```

Plusieurs instructions peuvent être écrites à la suite :

```text
afficher("Première ligne");
afficher("Deuxième ligne");
```

Les mots-clés, la ponctuation et les chaînes de caractères sont actuellement limités à cette syntaxe. Les erreurs sont encore signalées par des `panic!` et seront progressivement remplacées par une gestion d'erreurs dédiée.

## Documentation du langage

- [Documentation de la syntaxe algorithmique](https://github.com/BetterIUTInfoAix/DOC-ALGO-PAPIER)
- [Idée et contexte du projet](https://github.com/BetterIUTInfoAix/BetterAlgoPapier/issues/5)

## Feuille de route

- améliorer la gestion des erreurs ;
- lire le code source depuis un fichier ou l'entrée standard ;
- enrichir la syntaxe algorithmique ;
- ajouter d'autres générateurs de code ;
- compléter les tests du lexer, du parser et du code généré.
