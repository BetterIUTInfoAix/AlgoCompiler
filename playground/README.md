# Playground

Teste si un algorithme `.algo` est valide et voit le Python généré (ou
l'erreur localisée), directement dans le navigateur. La compilation se fait
localement en WebAssembly : aucun serveur de compilation requis.

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
  `{"ok":true,"python":"…"}` ou `{"ok":false,"code":"E101",…,"rendered":"…"}`.
- `pkg/` : module généré (`--target web`).
- `pkg-node/` : module généré (`--target nodejs`), utile pour tester le WASM
  sans navigateur : `node -e "import('./pkg-node/algo_playground.js').then(…)"`.
