import init, { compile_algo, run_algo } from "./pkg/algo_playground.js";

const editor = document.getElementById("editor");
const inputs = document.getElementById("inputs");
const output = document.getElementById("output");
const badge = document.getElementById("status");
const exampleSelect = document.getElementById("example");
const targetSelect = document.getElementById("target");
const runBtn = document.getElementById("run-btn");
const compileBtn = document.getElementById("compile-btn");
const copyBtn = document.getElementById("copy-btn");
const themeToggle = document.getElementById("theme-toggle");

// Thème clair / sombre (mémorisé, sinon système — comme le wiki).
themeToggle.addEventListener("click", () => {
  const next =
    document.documentElement.getAttribute("data-theme") === "dark" ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", next);
  try {
    window.localStorage.setItem("algoplayground-theme", next);
  } catch {
    /* stockage indisponible : le choix vaut pour la session */
  }
});

function setBadge(state, text) {
  badge.dataset.state = state;
  badge.textContent = text;
}

function showError(result) {
  const pos =
    result.line !== null && result.line !== undefined
      ? ` (ligne ${result.line}, colonne ${result.column})`
      : "";
  setBadge("error", `✗ Erreur [${result.code}]${pos}`);
  output.textContent = result.rendered || result.message;
  output.dataset.kind = "error";
  // Entrée `saisir` manquante : guider vers le champ Entrées.
  if (result.code === "E301" && (result.message || "").includes("aucune entrée fournie")) {
    inputs.classList.add("needed");
    inputs.focus();
  }
}

function wasmResult(call) {
  try {
    return JSON.parse(call());
  } catch (e) {
    setBadge("error", "✗ Erreur interne");
    output.textContent = "Le module WASM a renvoyé une réponse illisible : " + e;
    output.dataset.kind = "error";
    return null;
  }
}

/// Entrées : une ligne par `saisir` (les lignes vides comptent, sauf le
/// saut final du champ).
function readInputs() {
  const lines = inputs.value.replace(/\r/g, "").split("\n");
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

function run() {
  if (!editor.value.trim()) {
    setBadge("error", "✗ Rien à exécuter");
    output.textContent = "L'éditeur est vide : écrivez un algorithme ou choisissez un exemple.";
    output.dataset.kind = "error";
    return;
  }
  const result = wasmResult(() =>
    run_algo(editor.value, JSON.stringify(readInputs()), Date.now() % 4294967296),
  );
  if (!result) return;
  if (result.ok) {
    setBadge("ok", "✓ Exécuté");
    inputs.classList.remove("needed");
    output.textContent = result.output || "(aucune sortie : le programme n'a rien affiché)";
    output.dataset.kind = "run";
  } else {
    showError(result);
  }
}

function compile() {
  if (!editor.value.trim()) {
    setBadge("error", "✗ Rien à compiler");
    output.textContent = "L'éditeur est vide : écrivez un algorithme ou choisissez un exemple.";
    output.dataset.kind = "error";
    return;
  }
  const target = targetSelect.value;
  if (target !== "python") {
    setBadge("error", "✗ Cible indisponible");
    output.textContent = `La cible « ${target} » n'est pas encore supportée (bientôt).`;
    output.dataset.kind = "error";
    return;
  }
  const result = wasmResult(() => compile_algo(editor.value));
  if (!result) return;
  if (result.ok) {
    setBadge("ok", "✓ Valide — Python généré");
    output.textContent = result.python || "# (programme vide : rien à générer)";
    output.dataset.kind = "python";
  } else {
    showError(result);
  }
}

runBtn.addEventListener("click", run);
compileBtn.addEventListener("click", compile);
exampleSelect.addEventListener("change", () => {
  if (!exampleSelect.value) return;
  editor.value = EXAMPLES[exampleSelect.value] ?? "";
});
// Dès qu'on tape, l'exemple n'est plus « celui affiché ».
editor.addEventListener("input", () => {
  exampleSelect.value = "";
});
// Dès qu'on remplit les entrées, le guidage n'est plus nécessaire.
inputs.addEventListener("input", () => {
  inputs.classList.remove("needed");
});
copyBtn.addEventListener("click", async () => {
  if (!output.textContent) return;
  try {
    await navigator.clipboard.writeText(output.textContent);
    copyBtn.textContent = "Copié !";
  } catch {
    copyBtn.textContent = "Copie impossible";
  }
  setTimeout(() => (copyBtn.textContent = "Copier le résultat"), 1500);
});

// Démarrage vide : aucun exemple chargé, aucune compilation auto.
init()
  .then(() => {
    runBtn.disabled = false;
    compileBtn.disabled = false;
    setBadge("pending", "… prêt : écrivez ou choisissez un exemple");
  })
  .catch((e) => {
    setBadge("error", "✗ WASM non chargé");
    output.dataset.kind = "error";
    output.textContent =
      "Impossible de charger le compilateur WASM (" +
      e +
      ").\n\nServez ce dossier via HTTP au lieu d'ouvrir le fichier directement :\n  python3 -m http.server 8000\npuis ouvrez http://localhost:8000";
  });
