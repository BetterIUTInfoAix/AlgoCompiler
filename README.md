# AlgoCompiler

AlgoCompiler est un prototype de compilateur écrit en Rust pour le langage algorithmique utilisé dans le projet BetterAlgoPapier. Il transforme une instruction algorithmique en code Python exécutable.

Le projet est encore en développement. Pour le moment, il prend en charge l'affichage (`afficher`), les déclarations de variables (`declarer`), les affectations (`<-`) et les types primitifs scalaires (`entier`, `entier_naturel`, `reel`, `booleen`, `caractere`, `string`).

## Exemple

Code algorithmique :

```text
declarer unEntier : entier;
unEntier <- 42;
afficher("Bonjour, monde !");
afficher(unEntier);
```

Code Python généré :

```python
unEntier: int
unEntier = 42
print("Bonjour, monde !")
print(unEntier)
```

Un exemple complet est fourni dans `AlgoCompiler/exemples/hello.algo`.

## Prérequis

- [Rust et Cargo](https://www.rust-lang.org/tools/install)

Le projet utilise l'édition Rust 2024 et ne possède actuellement aucune dépendance externe.

## Installation et utilisation

Depuis le dossier `AlgoCompiler/` :

```bash
cargo run -- exemples/hello.algo
```

Cette commande compile le projet puis affiche le code Python généré pour le fichier source passé en argument. En cas d'erreur de compilation, un message localisé (ligne, colonne, extrait de code) est affiché sur `stderr`.

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
- `Expr` : représente une expression (littéral, variable, négation) ;
- `Type` : représente un type primitif ;
- `Program` : représente un programme complet ;
- `Statement::Afficher` : l'instruction `afficher` ;
- `Statement::Declarer` : l'instruction `declarer` ;
- `Statement::Affecter` : l'affectation `nom <- valeur ;`.

## Syntaxe supportée

### Affichage

```text
afficher("texte");
afficher(unEntier);
```

### Déclaration des variables

```text
declarer unEntier : entier;
declarer unNaturel : entier_naturel;
declarer unReel : reel;
declarer unBooleen : booleen;
declarer unCaractere : caractere;
declarer uneChaine : string;
```

Les déclarations sont traduites en annotations Python : `int` pour `entier`
et `entier_naturel`, `float` pour `reel`, `bool` pour `booleen`, et `str` pour
`caractere` et `string`. Une annotation ne crée pas de valeur et Python ne
vérifie pas le type à l'exécution ; elle sert à documenter le type et peut être
contrôlée par un outil comme mypy.

### Affectation

```text
unEntier <- 42;
unReel <- -5.61;
unBooleen <- vrai;
unCaractere <- 'e';
uneChaine <- "un mot";
```

### Littéraux

- entiers : `42`, `-5` ;
- réels : `3.14` (au moins un chiffre après le `.`) ;
- chaînes : `"du texte"` (guillemets doubles) ;
- caractères : `'a'`, `'1'`, `'\n'` (guillemets simples, échappements `\n`, `\t`, `\r`, `\\`, `\'`, `\"`) ;
- booléens : `vrai`, `faux`.

Les mots-clés sont insensibles à la casse (`DECLARER` vaut `declarer`), les identifiants conservent leur casse. Les commentaires `//` sont ignorés jusqu'à la fin de la ligne.

### Non supporté pour l'instant

- les tableaux (`tableau_de`) et les constantes (`constante`) ;
- les opérateurs arithmétiques (la négation `-` est acceptée) et booléens (`NON`, `OU`, `ET`) ;
- les structures de contrôle (`si`, `tant_que`, …) ;
- la vérification des types et de la portée des variables.

Les erreurs sont localisées (ligne, colonne) avec un extrait de code et une suggestion quand c'est possible ; leur gestion sera continuellement enrichie.

## Documentation du langage

- [Documentation de la syntaxe algorithmique](https://github.com/BetterIUTInfoAix/DOC-ALGO-PAPIER)
- [Idée et contexte du projet](https://github.com/BetterIUTInfoAix/BetterAlgoPapier/issues/5)

## Feuille de route

- améliorer la gestion des erreurs ;
- ajouter la lecture depuis l'entrée standard ;
- gérer les tableaux (`tableau_de`) et les constantes (`constante`) ;
- ajouter les opérateurs arithmétiques et booléens ;
- ajouter les structures de contrôle (`si`, `tant_que`, …) ;
- vérifier les types et la portée des variables ;
- ajouter d'autres générateurs de code ;
- compléter les tests du lexer, du parser et du code généré.
