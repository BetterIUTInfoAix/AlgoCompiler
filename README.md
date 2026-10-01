# AlgoCompiler

AlgoCompiler est un prototype de compilateur écrit en Rust pour le langage algorithmique utilisé dans le projet BetterAlgoPapier. Il transforme une instruction algorithmique en code Python exécutable.

Le projet est encore en développement. Pour le moment, il prend en charge l'affichage (`afficher`), les déclarations de variables (`declarer`, y compris `tableau_de` et `constante`), les affectations (`<-`), les expressions arithmétiques (`+`, `-`, `*`, `/`), les comparaisons (`vaut`, `ne_vaut_pas`, `<`, `>`, `<=`, `>=`), les opérateurs logiques (`et`, `et_alors`, `ou`, `ou_sinon`, `non`), les conditions (`si` / `sinon_si` / `sinon` / `fsi`, `choix_sur`) et les boucles (`boucle`, `repeter` / `jusqua`, `tant_que`, `pour`), avec génération de code Python.

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

### Vérifications de qualité

Avant de pousser une modification, formatez le code puis lancez l'analyse
Clippy avec les mêmes options que la CI :

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
```

`cargo fmt` applique le formatage standard, tandis que les options
`--all-targets`, `--all-features` et `-D warnings` permettent à Clippy de
transformer les avertissements en erreurs.

## Fonctionnement

Le compilateur est organisé en plusieurs étapes :

1. Le lexer (`src/lexer/`) transforme le texte en tokens.
2. Le parser (`src/parser/`) transforme les tokens en arbre syntaxique abstrait.
3. Le générateur (`src/codegen/`) produit le code Python à partir de cet arbre.

Les types principaux sont :

- `Token` : représente les éléments reconnus par le lexer ;
- `Expr` : représente une expression (littéral, variable, opération, appel, accès indicé) ;
- `Type` : représente un type (scalaire, `tableau_de` ou `constante`) ;
- `Program` : représente un programme complet ;
- `Statement::Afficher` : l'instruction `afficher` ;
- `Statement::Declarer` : l'instruction `declarer` (avec initialisation optionnelle) ;
- `Statement::Constante` : une constante déclarée (`constante … <- …`) ;
- `Statement::Affecter` : l'affectation `nom <- valeur ;` (y compris `tableau[indice] <- …`) ;
- `Statement::Si` / `Statement::ChoixSur` : les conditions ;
- `Statement::Boucle`, `Statement::Repeter`, `Statement::Jusqua`, `Statement::TantQue`, `Statement::Pour`, `Statement::Sortie`, `Statement::Continue` : les boucles.

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
declarer unCompteur : entier <- 0;
declarer notes : tableau_de 3 entier;
declarer Kpi : constante reel <- 3.14;
```

Les déclarations sont traduites en annotations Python : `int` pour `entier`
et `entier_naturel`, `float` pour `reel`, `bool` pour `booleen`, et `str` pour
`caractere` et `string`. Un tableau devient une `list` initialisée
(`t: list[int] = [0] * 3`). Une annotation ne crée pas de valeur et Python ne
vérifie pas le type à l'exécution ; elle sert à documenter le type et peut être
contrôlée par un outil comme mypy.

### Affectation

```text
unEntier <- 42;
unReel <- -5.61;
unBooleen <- vrai;
unCaractere <- 'e';
uneChaine <- "un mot";
notes[0] <- 12;
total <- prix + taxe * 2;
n <- taille(notes);
```

### Opérateurs

Arithmétique : `+`, `-`, `*`, `/` (priorités usuelles, parenthèses `(…)`).
Comparaison : `vaut` (`==`), `ne_vaut_pas` (`!=`), `<`, `>`, `<=`, `>=`.
Logique : `et` / `et_alors` (`and`), `ou` / `ou_sinon` (`or`), `non` (`not`,
prioritaire bas : `ou` < `et` < comparaison < arithmétique).
`taille(x)` donne la longueur (`len(x)` en Python).

### Conditions

```text
si (note >= 10) afficher("reussi");
sinon_si (note >= 16) afficher("tres bien");
sinon afficher("rate");
fsi

choix_sur jour entre
cas 1 : afficher("lundi");
cas 2 : afficher("mardi");
autre : afficher("autre jour");
fchoix
```

### Boucles

```text
pour (i variant_de 1 a 10) faire
afficher(i);
ffaire

pour (i variant_de 10 a 1 descendant) faire
afficher(i);
ffaire

tant_que (n > 0) faire
n <- n - 1;
ffaire

repeter
afficher(m);
jusqua (m vaut 3);

jusqua (m vaut 5) faire
afficher(m);
ffaire

boucle
si (k vaut 2) sortie;
fsi
fboucle
```

`sortie` interrompt la boucle (`break`), `continue` passe à l'itération
suivante. Une boucle `pour` parcourt ses bornes incluses.

### Littéraux

- entiers : `42`, `-5` ;
- réels : `3.14` (au moins un chiffre après le `.`) ;
- chaînes : `"du texte"` (guillemets doubles) ;
- caractères : `'a'`, `'1'`, `'\n'` (guillemets simples, échappements `\n`, `\t`, `\r`, `\\`, `\'`, `\"`) ;
- booléens : `vrai`, `faux`.

Les mots-clés sont insensibles à la casse (`DECLARER` vaut `declarer`), les identifiants conservent leur casse. Les commentaires `//` sont ignorés jusqu'à la fin de la ligne.

### Non supporté pour l'instant

- les sous-programmes (fonctions et procédures utilisateur) ;
- la vérification des types et de la portée des variables.

Les erreurs sont localisées (ligne, colonne) avec un extrait de code et une suggestion quand c'est possible ; leur gestion sera continuellement enrichie.

## Documentation du langage

- [Documentation de la syntaxe algorithmique](https://github.com/BetterIUTInfoAix/DOC-ALGO-PAPIER)
- [Idée et contexte du projet](https://github.com/BetterIUTInfoAix/BetterAlgoPapier/issues/5)

## Feuille de route

- améliorer la gestion des erreurs ;
- ajouter la lecture depuis l'entrée standard ;
- ajouter les sous-programmes (fonctions et procédures utilisateur) ;
- vérifier les types et la portée des variables ;
- ajouter d'autres générateurs de code ;
- compléter les tests du lexer, du parser et du code généré.
