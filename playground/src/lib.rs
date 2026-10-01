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
}
