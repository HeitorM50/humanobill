//! Utilidades de texto puras: normalização de descrições e formatação de valores.

use humanobill_core::Cents;

/// Minúsculas e sem acentos ("Música" -> "musica").
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

/// Palavras normalizadas: `fold` + quebra em tudo que não é letra ou dígito.
pub fn tokens(s: &str) -> Vec<String> {
    fold(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Descrição normalizada para comparação ("Seguro  Tela!" == "seguro tela").
pub fn normalized(s: &str) -> String {
    tokens(s).join(" ")
}

/// `true` se alguma palavra é `stem` ou o plural simples `stem + "s"`.
pub fn has_word(tokens: &[String], stem: &str) -> bool {
    tokens.iter().any(|t| {
        t == stem || (t.len() == stem.len() + 1 && t.starts_with(stem) && t.ends_with('s'))
    })
}

/// Formata centavos em reais: 123456 -> "R$ 1.234,56".
pub fn brl(cents: Cents) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let reais = (abs / 100).to_string();
    let centavos = abs % 100;
    let mut grouped = String::new();
    for (i, ch) in reais.chars().enumerate() {
        if i > 0 && (reais.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{sign}R$ {grouped},{centavos:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_remove_acentos_e_caixa() {
        assert_eq!(
            fold("Música CONTEÚDO Proteção Assistência"),
            "musica conteudo protecao assistencia"
        );
    }

    #[test]
    fn normalizado_ignora_pontuacao_e_espacos() {
        assert_eq!(normalized("  Seguro   Tela! "), "seguro tela");
        assert_eq!(normalized("SEGURO-TELA"), "seguro tela");
    }

    #[test]
    fn palavra_aceita_plural_simples_mas_nao_prefixo() {
        let t = tokens("Jogos e Bancários");
        assert!(has_word(&t, "jogo"));
        assert!(!has_word(&t, "banca"));
    }

    #[test]
    fn formata_reais() {
        assert_eq!(brl(1290), "R$ 12,90");
        assert_eq!(brl(5), "R$ 0,05");
        assert_eq!(brl(123_456), "R$ 1.234,56");
        assert_eq!(brl(100_000_000), "R$ 1.000.000,00");
        assert_eq!(brl(-300), "-R$ 3,00");
    }
}
