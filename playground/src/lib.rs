use wasm_bindgen::prelude::*;

/// Compile du source `.algo` et renvoie le résultat en JSON.
///
/// Succès : `{"ok":true,"python":"..."}`
/// Échec : `{"ok":false,"code":"E101","phase":"syntaxe","line":3,"column":5,
///           "message":"…","hint":"…"|null,"rendered":"…"}`
///
/// `rendered` contient le message complet avec l'extrait de code et le
/// curseur `^^^`, prêt à afficher dans un `<pre>`.
#[wasm_bindgen]
pub fn compile_algo(source: &str) -> String {
    let tokens = match betteralgo::lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(err) => return error_json(source, &err),
    };
    let program = match betteralgo::parser::parse(&tokens) {
        Ok(program) => program,
        Err(err) => return error_json(source, &err),
    };
    let python = betteralgo::codegen::generate_python(&program);
    format!("{{\"ok\":true,\"python\":\"{}\"}}", json_escape(&python))
}

/// Exécute un source `.algo` et renvoie le résultat en JSON.
///
/// `inputs_json` est un tableau JSON de chaînes (une par `saisir`) :
/// `["bonjour","14"]`. `seed` amorce le tirage de `rand` (ex : `Date.now()`).
///
/// Succès : `{"ok":true,"output":"..."}`
/// Échec de compilation : même format que [`compile_algo`].
/// Échec d'exécution : `{"ok":false,"code":"E301","phase":"exécution",
///   "line":null,"column":null,"message":"…","hint":null,"rendered":"…"}`
#[wasm_bindgen]
pub fn run_algo(source: &str, inputs_json: &str, seed: u32) -> String {
    let inputs = match parse_string_array(inputs_json) {
        Ok(inputs) => inputs,
        Err(message) => return runtime_error_json(&message),
    };
    let tokens = match betteralgo::lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(err) => return error_json(source, &err),
    };
    let program = match betteralgo::parser::parse(&tokens) {
        Ok(program) => program,
        Err(err) => return error_json(source, &err),
    };
    // Mélange la graine JS (sinon `Date.now()` varierait trop peu).
    let mixed = (seed as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(0xBF58_476D_1CE4_E5B9)
        | 1;
    match betteralgo::interp::run_with_seed(&program, &inputs, mixed) {
        Ok(output) => format!("{{\"ok\":true,\"output\":\"{}\"}}", json_escape(&output)),
        Err(err) => runtime_error_json(&err.message),
    }
}

fn runtime_error_json(message: &str) -> String {
    format!(
        "{{\"ok\":false,\"code\":\"E301\",\"phase\":\"exécution\",\"line\":null,\"column\":null,\"message\":\"{}\",\"hint\":null,\"rendered\":\"Erreur d'exécution : {}\"}}",
        json_escape(message),
        json_escape(message),
    )
}

/// Parse un tableau JSON de chaînes (`["a","b"]`, sans serde).
/// Inutilisable tel quel → message d'erreur pour l'appelant.
fn parse_string_array(json: &str) -> Result<Vec<String>, String> {
    let chars: Vec<char> = json.chars().collect();
    let mut pos = 0;
    let err = || "entrées illisibles (tableau JSON de chaînes attendu)".to_string();
    let skip_ws = |pos: &mut usize| {
        while chars.get(*pos).is_some_and(|c| c.is_whitespace()) {
            *pos += 1;
        }
    };
    skip_ws(&mut pos);
    if chars.get(pos) != Some(&'[') {
        return Err(err());
    }
    pos += 1;
    let mut out = Vec::new();
    // Après une virgule, une chaîne doit suivre (pas de virgule traînante).
    let mut need_string = false;
    loop {
        skip_ws(&mut pos);
        match chars.get(pos) {
            None => return Err(err()),
            Some(']') => {
                if need_string {
                    return Err(err());
                }
                pos += 1;
                skip_ws(&mut pos);
                return if pos == chars.len() {
                    Ok(out)
                } else {
                    Err(err())
                };
            }
            Some('"') => {
                pos += 1;
                let mut s = String::new();
                loop {
                    match chars.get(pos) {
                        None => return Err(err()),
                        Some('"') => {
                            pos += 1;
                            break;
                        }
                        Some('\\') => {
                            pos += 1;
                            match chars.get(pos) {
                                Some('"') => s.push('"'),
                                Some('\\') => s.push('\\'),
                                Some('n') => s.push('\n'),
                                Some('r') => s.push('\r'),
                                Some('t') => s.push('\t'),
                                _ => return Err(err()),
                            }
                            pos += 1;
                        }
                        Some(c) => {
                            s.push(*c);
                            pos += 1;
                        }
                    }
                }
                out.push(s);
                need_string = false;
                skip_ws(&mut pos);
                match chars.get(pos) {
                    Some(',') => {
                        pos += 1;
                        need_string = true;
                    }
                    Some(']') => {}
                    _ => return Err(err()),
                }
            }
            _ => return Err(err()),
        }
    }
}

fn error_json(source: &str, err: &betteralgo::errors::CompileError) -> String {
    let (line, column) = match err.span {
        Some(span) => (span.line.to_string(), span.column.to_string()),
        None => ("null".to_string(), "null".to_string()),
    };
    let hint = match &err.hint {
        Some(hint) => format!("\"{}\"", json_escape(hint)),
        None => "null".to_string(),
    };
    format!(
        "{{\"ok\":false,\"code\":\"{}\",\"phase\":\"{}\",\"line\":{line},\"column\":{column},\"message\":\"{}\",\"hint\":{hint},\"rendered\":\"{}\"}}",
        err.code(),
        err.kind.phase(),
        json_escape(&err.kind.message()),
        json_escape(&err.render(source, None)),
    )
}

/// Échappe une chaîne pour l'inclure dans du JSON (sans serde).
fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programme_valide_renvoie_du_python() {
        let out = compile_algo("declarer x : entier;\nx <- 42;\nafficher(x);\n");
        assert!(out.contains("\"ok\":true"), "{out}");
        assert!(out.contains("print(x)"), "{out}");
    }

    #[test]
    fn programme_invalide_renvoie_une_erreur_localisee() {
        let out = compile_algo("affichr(\"x\");");
        assert!(out.contains("\"ok\":false"), "{out}");
        assert!(out.contains("\"code\":\"E101\""), "{out}");
        assert!(out.contains("vouliez-vous dire"), "{out}");
    }

    #[test]
    fn json_echappe_guillemets_et_retours() {
        assert_eq!(json_escape("a\"b\nc\\d"), "a\\\"b\\nc\\\\d");
    }

    #[test]
    fn execution_renvoie_la_sortie() {
        let out = run_algo("afficher(1 + 2);", "[]", 7);
        assert!(out.contains("\"ok\":true"), "{out}");
        assert!(out.contains("3\\n"), "{out}");
    }

    #[test]
    fn execution_avec_entrees() {
        let out = run_algo(
            "declarer n : entier; saisir(n); afficher(n);",
            "[\"14\"]",
            7,
        );
        assert!(out.contains("\"ok\":true"), "{out}");
        assert!(out.contains("14\\n"), "{out}");
    }

    #[test]
    fn execution_erreur_runtime() {
        let out = run_algo("boucle fboucle", "[]", 7);
        assert!(out.contains("\"ok\":false"), "{out}");
        assert!(out.contains("\"code\":\"E301\""), "{out}");
        assert!(out.contains("boucle infinie"), "{out}");
    }

    #[test]
    fn execution_erreur_compile() {
        let out = run_algo("affichr(\"x\");", "[]", 7);
        assert!(out.contains("\"ok\":false"), "{out}");
        assert!(out.contains("\"code\":\"E101\""), "{out}");
    }

    #[test]
    fn tableau_json_invalide() {
        let out = run_algo("afficher(1);", "pas du json", 7);
        assert!(out.contains("\"ok\":false"), "{out}");
        assert!(out.contains("\"code\":\"E301\""), "{out}");
    }

    #[test]
    fn parse_tableau_json() {
        assert_eq!(
            parse_string_array("[\"a\",\"b\\nc\"]").unwrap(),
            vec!["a".to_string(), "b\nc".to_string()]
        );
        assert_eq!(parse_string_array("[]").unwrap(), Vec::<String>::new());
        assert!(parse_string_array("[\"a\",]").is_err());
        assert!(parse_string_array("x").is_err());
    }
}
