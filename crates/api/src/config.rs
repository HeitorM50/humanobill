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
