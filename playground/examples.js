const EXAMPLES = {
  saisir: `declarer input : string;
afficher ("veuillez saisir le mot/texte");
saisir(input);
pour (i variant_de 0 à 2)
faire
afficher (input);
ffaire`,
  tout: `// Test de tout le langage : types, operateurs, conditions, boucles.

// --- Declarations : tous les types, init inline, constante, tableau ---
declarer e : entier <- 20;
declarer n : entier_naturel <- 7;
declarer r : reel <- 2.5;
declarer b : booleen <- vrai;
declarer c : caractere <- 'z';
declarer s : string <- "algo";
declarer Kpi : constante reel <- 3.14;
declarer t : tableau_de 4 entier;
t[0] <- 5;
t[1] <- 10;
t[2] <- 15;
t[3] <- e + n;

afficher(e);
afficher(n);
afficher(r);
afficher(b);
afficher(c);
afficher(s);
afficher(Kpi);
afficher(t[3]);

// --- Arithmetique, comparaisons, logique, appels ---
declarer calc : reel;
calc <- (e + n) * 2 - r / 2;
afficher(calc);
afficher(taille(t));
afficher(taille(s));

si ((e vaut 20) et (n ne_vaut_pas 0)) afficher("et ok");
fsi
si ((e < 10) ou (e > 10)) afficher("ou ok");
fsi
si ((e >= 20) et_alors (n <= 7)) afficher("et_alors ok");
fsi
si ((e < 0) ou_sinon (n vaut 7)) afficher("ou_sinon ok");
fsi
si (non (e vaut 0)) afficher("non ok");
fsi

// --- Conditions : si / sinon_si / sinon + choix_sur ---
si (e >= 100) afficher("grand");
sinon_si (e >= 10) afficher("moyen");
sinon afficher("petit");
fsi

choix_sur n entre
cas 1 : afficher("un");
cas 7 : afficher("sept");
autre : afficher("autre");
fchoix

// --- Boucles : pour, tant_que, repeter, jusqua, boucle ---
declarer i : entier;
pour (i variant_de 1 a 3) faire
afficher(i);
ffaire

pour (i variant_de 3 a 1 descendant) faire
afficher(t[i - 1]);
ffaire

declarer k : entier;
k <- 2;
tant_que (k > 0) faire
afficher(k);
k <- k - 1;
ffaire

repeter
afficher(k);
k <- k + 1;
jusqua (k vaut 2);

jusqua (k vaut 4) faire
afficher(k);
k <- k + 1;
ffaire

boucle
k <- k + 1;
si (k vaut 6) sortie;
fsi
si (k vaut 5) continue;
fsi
afficher(k);
fboucle

afficher("fini");`,
  hello: `afficher("Salut !!");
afficher("Test !");
afficher("Je suis une String !");`,
  var: `// Test de tous les types primitifs supportés.
declarer testVarEntier : entier;
declarer testVarEntierNaturel : entier_naturel;
declarer testVarReel : reel;
declarer testVarBooleen : booleen;
declarer testVarCaractere : caractere;
declarer testVarStr : string;

testVarEntier <- 42;
testVarEntierNaturel <- 7;
testVarReel <- 3.14;
testVarBooleen <- vrai;
testVarCaractere <- 'a';
testVarStr <- "Bonjour";

afficher("Entier : ");
afficher(testVarEntier);
afficher("Entier naturel : ");
afficher(testVarEntierNaturel);
afficher("Réel : ");
afficher(testVarReel);
afficher("Booléen : ");
afficher(testVarBooleen);
afficher("Caractère : ");
afficher(testVarCaractere);
afficher("Chaîne : ");
afficher(testVarStr);`,
  conditions: `// Test des conditions : si / sinon_si / sinon / fsi.
declarer note : entier;
note <- 12;

si (note >= 10) afficher("reussi");
sinon afficher("rate");
fsi

si (note >= 16) afficher("tres bien");
sinon_si (note >= 10) afficher("passable");
sinon afficher("insuffisant");
fsi

si ((note >= 10) et (note < 20)) afficher("dans l'intervalle");
fsi

si ((note < 0) ou (note > 20)) afficher("hors bornes");
sinon afficher("note valide");
fsi

si (non (note vaut 0)) afficher("note non nulle");
fsi

// Test du choix_sur.
declarer jour : entier;
jour <- 2;
choix_sur jour entre
cas 1 : afficher("lundi");
cas 2 : afficher("mardi");
autre : afficher("autre jour");
fchoix`,
  boucles: `// Test des boucles : pour, tant_que, repeter, jusqua, boucle.
declarer i : entier;

// Boucle pour croissante (1..5 inclus).
pour (i variant_de 1 a 5) faire
afficher(i);
ffaire

// Boucle pour decroissante (5..1 inclus).
pour (i variant_de 5 a 1 descendant) faire
afficher(i);
ffaire

// Boucle tant_que : compte a rebours.
declarer n : entier;
n <- 3;
tant_que (n > 0) faire
afficher(n);
n <- n - 1;
ffaire

// Boucle repeter / jusqua : execute au moins une fois.
declarer m : entier;
m <- 0;
repeter
afficher(m);
m <- m + 1;
jusqua (m vaut 3);

// Boucle jusqua / faire : teste avant d'executer.
jusqua (m vaut 5) faire
afficher(m);
m <- m + 1;
ffaire

// Boucle infinie avec sortie.
declarer k : entier;
k <- 0;
boucle
afficher(k);
k <- k + 1;
si (k vaut 2) sortie;
fsi
fboucle`,
  tableau: `// Test des tableaux et constantes.
declarer t : tableau_de 3 entier;
t[0] <- 6;
t[1] <- 7;
t[2] <- 8;
afficher(t[0]);
afficher(t[1]);
afficher(t[2]);

declarer Kpi : constante reel <- 3.14;
afficher(Kpi);

// Parcours de tableau avec la taille.
declarer j : entier;
pour (j variant_de 0 a taille(t) - 1) faire
afficher(t[j]);
ffaire`,
};
