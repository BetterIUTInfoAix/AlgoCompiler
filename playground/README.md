# Playground

Écrivez un algorithme `.algo` et **exécutez-le** directement dans le
navigateur (avec vos entrées pour `saisir`), ou **compilez-le** vers un
langage cible (Python pour l'instant, d'autres à venir). En cas d'erreur,
le message localisé (ou d'exécution) s'affiche. Tout tourne localement en
WebAssembly : aucun serveur de compilation requis.

Au démarrage, l'éditeur est vide : choisissez un exemple dans le menu ou
écrivez votre code, puis cliquez sur **Exécuter** ou **Compiler** (aucune
compilation automatique).

## Lancer

```bash
./build.sh
python3 -m http.server 8000
```

Puis ouvrez `http://localhost:8000`.

`./build.sh` installe `wasm-pack` (dossier `node_modules/`, ignoré par git)
puis génère le module dans `pkg/` (ignoré par git, à reconstruire après
chaque modification du compilateur).

## Déploiement sur GitHub Pages

Le workflow `.github/workflows/playground.yml` s'en occupe à chaque push sur
`main` (touchant `playground/` ou `AlgoCompiler/`) :

1. tests du pont WASM (`cargo test` dans `playground/`) ;
2. build du module (`wasm-pack build --target web`) ;
3. staging de `index.html`, `style.css`, `app.js`, `examples.js` + `pkg/`
   dans `site/` (sans `node_modules` ni `target`) ;
4. déploiement de `site/` via `actions/deploy-pages` (les PR ne font que
   builder, sans déployer).

Activation (une seule fois) : repo → Settings → Pages → Source :
**GitHub Actions**. L'URL sera `https://<org>.github.io/AlgoCompiler/`.
Tous les chemins sont relatifs, donc le sous-chemin du projet fonctionne,
et Pages sert le `.wasm` en `application/wasm`.
Pensez à commiter `playground/package-lock.json` (`npm ci` en a besoin).

## Contenu

- `index.html`, `style.css`, `app.js` : l'interface (vanilla JS, responsive
  PC / mobile, aucune dépendance externe).
- `examples.js` : les exemples intégrés (recopies de `../AlgoCompiler/exemples/`).
- `src/lib.rs` : le pont WASM — expose `compile_algo(source)` qui renvoie
  `{"ok":true,"python":"…"}` ou `{"ok":false,"code":"E101",…,"rendered":"…"}`,
  et `run_algo(source, inputs_json)` qui exécute le programme (`inputs_json`
  : tableau JSON de chaînes, une par `saisir`) et renvoie
  `{"ok":true,"output":"…"}` ou une erreur (`E101`/`E102` compilation,
  `E301` exécution).
- `pkg/` : module généré (`--target web`).
- `pkg-node/` : module généré (`--target nodejs`), utile pour tester le WASM
  sans navigateur : `node -e "import('./pkg-node/algo_playground.js').then(…)"`.
