//! Configuração lida do ambiente. Falha cedo se faltar algo obrigatório.

use std::path::PathBuf;

use axum::http::HeaderValue;
use humanobill_extract::GeminiConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub cors_origins: Vec<HeaderValue>,
    /// `None` = upload real desligado (a demo continua funcionando).
    pub gemini: Option<GeminiConfig>,
    pub fixtures_dir: PathBuf,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("variável obrigatória ausente: {0}")]
    Missing(&'static str),
    #[error("valor inválido em {0}")]
    Invalid(&'static str),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|k| std::env::var(k).ok().filter(|v| !v.trim().is_empty()))
    }

    /// Versão testável: recebe a função de leitura das variáveis.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let port = match get("PORT") {
            Some(p) => p.parse().map_err(|_| ConfigError::Invalid("PORT"))?,
            None => 8080,
        };
        let database_url = get("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;
        let cors_origins = get("CORS_ORIGIN")
            .ok_or(ConfigError::Missing("CORS_ORIGIN"))?
            .split(',')
            .map(|o| {
                // O navegador manda a origem sem barra final: aceita "https://x.app/" no env.
                HeaderValue::from_str(o.trim().trim_end_matches('/'))
                    .map_err(|_| ConfigError::Invalid("CORS_ORIGIN"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let gemini = get("GEMINI_API_KEY").map(|api_key| GeminiConfig {
            api_key,
            model: get("GEMINI_MODEL").unwrap_or_else(|| GeminiConfig::DEFAULT_MODEL.into()),
            fallback_model: Some(
                get("GEMINI_FALLBACK_MODEL")
                    .unwrap_or_else(|| GeminiConfig::DEFAULT_FALLBACK_MODEL.into()),
            ),
            base_url: get("GEMINI_BASE_URL")
                .unwrap_or_else(|| GeminiConfig::DEFAULT_BASE_URL.into()),
        });
        let fixtures_dir = get("FIXTURES_DIR")
            .unwrap_or_else(|| "fixtures".into())
            .into();
        Ok(Self {
            port,
            database_url,
            cors_origins,
            gemini,
            fixtures_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup<'a>(m: &'a HashMap<&str, &str>) -> impl Fn(&str) -> Option<String> + 'a {
        |k| m.get(k).map(|v| v.to_string())
    }

    #[test]
    fn config_minima_valida() {
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app, http://localhost:5173"),
        ]);
        let c = Config::from_lookup(lookup(&m)).unwrap();
        assert_eq!(c.port, 8080);
        assert_eq!(c.cors_origins.len(), 2);
        assert!(c.gemini.is_none());
    }

    #[test]
    fn cors_ignora_barra_final_e_espacos() {
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            (
                "CORS_ORIGIN",
                " https://humanobill.vercel.app/ ,http://localhost:5173/",
            ),
        ]);
        let c = Config::from_lookup(lookup(&m)).unwrap();
        assert_eq!(c.cors_origins[0], "https://humanobill.vercel.app");
        assert_eq!(c.cors_origins[1], "http://localhost:5173");
    }

    #[test]
    fn falta_database_url() {
        let m = HashMap::from([("CORS_ORIGIN", "https://a.app")]);
        assert_eq!(
            Config::from_lookup(lookup(&m)).unwrap_err(),
            ConfigError::Missing("DATABASE_URL")
        );
    }

    #[test]
    fn falta_cors_origin() {
        let m = HashMap::from([("DATABASE_URL", "postgres://x")]);
        assert_eq!(
            Config::from_lookup(lookup(&m)).unwrap_err(),
            ConfigError::Missing("CORS_ORIGIN")
        );
    }

    #[test]
    fn porta_invalida() {
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app"),
            ("PORT", "abc"),
        ]);
        assert_eq!(
            Config::from_lookup(lookup(&m)).unwrap_err(),
            ConfigError::Invalid("PORT")
        );
    }

    #[test]
    fn gemini_com_modelo_padrao() {
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app"),
            ("GEMINI_API_KEY", "k"),
        ]);
        let c = Config::from_lookup(lookup(&m)).unwrap();
        assert_eq!(c.gemini.unwrap().model, GeminiConfig::DEFAULT_MODEL);
    }
}

#[cfg(test)]
mod edge_tests {
    use std::collections::HashMap;

    use super::*;

    fn base() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app"),
        ])
    }

    fn cfg(m: &HashMap<&str, &str>) -> Result<Config, ConfigError> {
        Config::from_lookup(|k| m.get(k).map(|v| v.to_string()))
    }

    #[test]
    fn porta_nos_limites_do_u16() {
        for (txt, esperado) in [("0", 0), ("1", 1), ("65535", 65535), ("08080", 8080)] {
            let mut m = base();
            m.insert("PORT", txt);
            assert_eq!(cfg(&m).unwrap().port, esperado, "PORT={txt}");
        }
    }

    #[test]
    fn porta_invalida_em_varios_formatos() {
        for txt in [
            "65536",
            "-1",
            "1.5",
            "8080 ",
            " 8080",
            "8_080",
            "٨٠٨٠",
            "0x1F90",
            "1e3",
            "porta",
            "",
            "99999999999999999999",
        ] {
            let mut m = base();
            m.insert("PORT", txt);
            assert_eq!(
                cfg(&m).unwrap_err(),
                ConfigError::Invalid("PORT"),
                "PORT={txt:?}"
            );
        }
    }

    #[test]
    fn ordem_de_validacao_porta_depois_banco_depois_cors() {
        let m = HashMap::from([("PORT", "abc")]);
        assert_eq!(cfg(&m).unwrap_err(), ConfigError::Invalid("PORT"));
        let m = HashMap::new();
        assert_eq!(cfg(&m).unwrap_err(), ConfigError::Missing("DATABASE_URL"));
    }

    #[test]
    fn mensagens_de_erro_em_portugues_citam_a_variavel() {
        assert_eq!(
            ConfigError::Missing("DATABASE_URL").to_string(),
            "variável obrigatória ausente: DATABASE_URL"
        );
        assert_eq!(
            ConfigError::Invalid("PORT").to_string(),
            "valor inválido em PORT"
        );
    }

    #[test]
    fn cors_com_barras_repetidas_e_varias_origens() {
        let mut m = base();
        m.insert(
            "CORS_ORIGIN",
            "https://a.app///,  http://localhost:5173/ ,https://b.app",
        );
        let c = cfg(&m).unwrap();
        let origens: Vec<&str> = c.cors_origins.iter().map(|o| o.to_str().unwrap()).collect();
        assert_eq!(
            origens,
            ["https://a.app", "http://localhost:5173", "https://b.app"]
        );
    }

    #[test]
    fn cors_com_quebra_de_linha_ou_controle_e_invalido_contra_injecao_de_cabecalho() {
        for origem in [
            "https://a.app\r\nX-Evil: 1",
            "https://a.app\nX-Evil: 1",
            "https://a.app\u{7f}",
            "https://a.app\u{0}",
        ] {
            let mut m = base();
            m.insert("CORS_ORIGIN", origem);
            assert_eq!(
                cfg(&m).unwrap_err(),
                ConfigError::Invalid("CORS_ORIGIN"),
                "{origem:?}"
            );
        }
    }

    #[test]
    fn gemini_so_liga_com_chave_e_usa_padroes_para_o_resto() {
        let mut m = base();
        m.insert("GEMINI_MODEL", "modelo-x"); // sem chave: ignorado
        m.insert("GEMINI_FALLBACK_MODEL", "fb");
        assert!(cfg(&m).unwrap().gemini.is_none());

        m.remove("GEMINI_MODEL");
        m.remove("GEMINI_FALLBACK_MODEL");
        m.insert("GEMINI_API_KEY", "k");
        let g = cfg(&m).unwrap().gemini.unwrap();
        assert_eq!(g.api_key, "k");
        assert_eq!(g.model, GeminiConfig::DEFAULT_MODEL);
        assert_eq!(
            g.fallback_model.as_deref(),
            Some(GeminiConfig::DEFAULT_FALLBACK_MODEL)
        );
        assert_eq!(g.base_url, GeminiConfig::DEFAULT_BASE_URL);
    }

    #[test]
    fn gemini_com_tudo_customizado_e_chave_com_espacos_e_unicode_preservada() {
        let mut m = base();
        m.insert("GEMINI_API_KEY", " chave ção 🔑 ");
        m.insert("GEMINI_MODEL", "m1");
        m.insert("GEMINI_FALLBACK_MODEL", "m2,m3");
        m.insert("GEMINI_BASE_URL", "http://127.0.0.1:9999");
        let g = cfg(&m).unwrap().gemini.unwrap();
        assert_eq!(g.api_key, " chave ção 🔑 ");
        assert_eq!(g.model, "m1");
        assert_eq!(g.fallback_model.as_deref(), Some("m2,m3"));
        assert_eq!(g.base_url, "http://127.0.0.1:9999");
    }

    #[test]
    fn pasta_de_fixtures_padrao_e_customizada() {
        assert_eq!(
            cfg(&base()).unwrap().fixtures_dir,
            PathBuf::from("fixtures")
        );
        let mut m = base();
        m.insert("FIXTURES_DIR", "/tmp/pasta com espaço/ação");
        assert_eq!(
            cfg(&m).unwrap().fixtures_dir,
            PathBuf::from("/tmp/pasta com espaço/ação")
        );
    }

    #[test]
    fn database_url_e_repassada_sem_alteracao() {
        let mut m = base();
        m.insert(
            "DATABASE_URL",
            "postgres://u:p%40ss@h:5432/db?sslmode=require",
        );
        assert_eq!(
            cfg(&m).unwrap().database_url,
            "postgres://u:p%40ss@h:5432/db?sslmode=require"
        );
    }
}
