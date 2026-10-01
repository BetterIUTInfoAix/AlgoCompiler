#!/bin/sh
# Construit le module WASM du playground.
# Usage : ./build.sh   (depuis le dossier playground/)
set -e
cd "$(dirname "$0")"
npm install
npx wasm-pack build --target web --out-dir pkg
echo "OK : servez le dossier playground avec, par ex. : python3 -m http.server 8000"
