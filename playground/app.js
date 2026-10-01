import init, { compile_algo } from "./pkg/algo_playground.js";

const editor = document.getElementById("editor");
const output = document.getElementById("output");
const badge = document.getElementById("status");
const exampleSelect = document.getElementById("example");
const compileBtn = document.getElementById("compile-btn");
const copyBtn = document.getElementById("copy-btn");

let lastPython = "";
let debounce = null;

function setBadge(state, text) {
  badge.dataset.state = state;
  badge.textContent = text;
}

function compile() {
  let result;
  try {
    result = JSON.parse(compile_algo(editor.value));
  } catch (e) {
    setBadge("error", "✗ Erreur interne");
    output.textContent = "Le module WASM a renvoyé une réponse illisible : " + e;
    output.dataset.kind = "error";
    return;
  }
  if (result.ok) {
    lastPython = result.python;
    setBadge("ok", "✓ Valide — Python généré");
    output.textContent = result.python || "# (programme vide : rien à générer)";
    output.dataset.kind = "python";
  } else {
    lastPython = "";
    const pos =
      result.line !== null ? ` (ligne ${result.line}, colonne ${result.column})` : "";
    setBadge("error", `✗ Erreur [${result.code}]${pos}`);
    output.textContent = result.rendered;
    output.dataset.kind = "error";
  }
}

function scheduleCompile() {
  clearTimeout(debounce);
  debounce = setTimeout(compile, 400);
}

compileBtn.addEventListener("click", compile);
editor.addEventListener("input", scheduleCompile);
exampleSelect.addEventListener("change", () => {
  editor.value = EXAMPLES[exampleSelect.value] ?? "";
  compile();
});
copyBtn.addEventListener("click", async () => {
  if (!lastPython) return;
  try {
    await navigator.clipboard.writeText(lastPython);
    copyBtn.textContent = "Copié !";
  } catch {
    copyBtn.textContent = "Copie impossible";
  }
  setTimeout(() => (copyBtn.textContent = "Copier le Python"), 1500);
});

// Exemple par défaut.
editor.value = EXAMPLES.tout;

init()
  .then(() => {
    compileBtn.disabled = false;
    compile();
  })
  .catch((e) => {
    setBadge("error", "✗ WASM non chargé");
    output.dataset.kind = "error";
    output.textContent =
      "Impossible de charger le compilateur WASM (" +
      e +
      ").\n\nServez ce dossier via HTTP au lieu d'ouvrir le fichier directement :\n  python3 -m http.server 8000\npuis ouvrez http://localhost:8000";
  });
