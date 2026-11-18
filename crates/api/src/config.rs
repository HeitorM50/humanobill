//! Configuração lida do ambiente. Falha cedo se faltar algo obrigatório.

use std::path::PathBuf;

use axum::http::HeaderValue;
use humanobill_extract::GeminiConfig;

use crate::mailer::ResendConfig;

/// Segredo que nunca aparece em `Debug`/log.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(<oculto>)")
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub cors_origins: Vec<HeaderValue>,
    /// `None` = upload real desligado (a demo continua funcionando).
    pub gemini: Option<GeminiConfig>,
    pub fixtures_dir: PathBuf,
    /// `None` = lembrete por e-mail desligado (RF18); o resto da API segue normal.
    pub resend: Option<ResendConfig>,
    /// Protege `POST /internal/reminders/dispatch`. `None` = rota desligada.
    pub cron_secret: Option<Secret>,
    /// Endereço público do front, usado nos links dos e-mails (sem barra final).
    pub web_url: String,
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
        // Lembrete só liga com chave E remetente: não presume domínio (P11).
        // Vazio ou só espaços = não configurado (igual ao `from_env`), mesmo vindo direto do lookup.
        let not_blank = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let resend = match (not_blank("RESEND_API_KEY"), not_blank("RESEND_FROM")) {
            (Some(api_key), Some(from)) => Some(ResendConfig {
                api_key,
                from,
                base_url: get("RESEND_BASE_URL")
                    .unwrap_or_else(|| ResendConfig::DEFAULT_BASE_URL.into()),
            }),
            _ => None,
        };
        let cron_secret = not_blank("CRON_SECRET").map(Secret);
        // Sem WEB_URL, usa a primeira origem do CORS (o front).
        let web_url = match get("WEB_URL") {
            Some(u) => u,
            None => cors_origins
                .first()
                .and_then(|o| o.to_str().ok())
                .unwrap_or_default()
                .to_string(),
        }
        .trim()
        .trim_end_matches('/')
        .to_string();
        Ok(Self {
            port,
            database_url,
            cors_origins,
            gemini,
            fixtures_dir,
            resend,
            cron_secret,
            web_url,
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
    fn lembrete_desligado_sem_chave_ou_sem_remetente() {
        let base = [
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app"),
        ];
        let c = Config::from_lookup(lookup(&HashMap::from(base))).unwrap();
        assert!(c.resend.is_none() && c.cron_secret.is_none());
        let so_chave = HashMap::from([base[0], base[1], ("RESEND_API_KEY", "re_x")]);
        assert!(
            Config::from_lookup(lookup(&so_chave))
                .unwrap()
                .resend
                .is_none()
        );
        let so_from = HashMap::from([base[0], base[1], ("RESEND_FROM", "a@b.c")]);
        assert!(
            Config::from_lookup(lookup(&so_from))
                .unwrap()
                .resend
                .is_none()
        );
    }

    #[test]
    fn segredo_e_chaves_vazios_ou_so_com_espacos_contam_como_nao_configurados() {
        for vazio in ["", "   ", "\t\n"] {
            let m = HashMap::from([
                ("DATABASE_URL", "postgres://x"),
                ("CORS_ORIGIN", "https://a.app"),
                ("RESEND_API_KEY", vazio),
                ("RESEND_FROM", vazio),
                ("CRON_SECRET", vazio),
            ]);
            let c = Config::from_lookup(lookup(&m)).unwrap();
            assert!(c.resend.is_none(), "{vazio:?}");
            assert!(c.cron_secret.is_none(), "{vazio:?}");
        }
        // Um só vazio já desliga o envio: não presume remetente nem chave.
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app"),
            ("RESEND_API_KEY", "re_x"),
            ("RESEND_FROM", "  "),
        ]);
        assert!(Config::from_lookup(lookup(&m)).unwrap().resend.is_none());
    }

    #[test]
    fn lembrete_liga_com_chave_e_remetente_do_ambiente() {
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "https://a.app/"),
            ("RESEND_API_KEY", "re_x"),
            ("RESEND_FROM", "HumanoBill <oi@exemplo.com>"),
            ("CRON_SECRET", "segredo"),
        ]);
        let c = Config::from_lookup(lookup(&m)).unwrap();
        let r = c.resend.unwrap();
        assert_eq!(r.from, "HumanoBill <oi@exemplo.com>");
        assert_eq!(r.base_url, ResendConfig::DEFAULT_BASE_URL);
        assert_eq!(c.cron_secret, Some(Secret("segredo".into())));
        assert_eq!(c.web_url, "https://a.app");
    }

    #[test]
    fn web_url_explicita_vence_o_cors_e_segredos_ficam_fora_do_debug() {
        let m = HashMap::from([
            ("DATABASE_URL", "postgres://x"),
            ("CORS_ORIGIN", "http://localhost:5173"),
            ("WEB_URL", "https://humanobill.vercel.app/"),
            ("RESEND_API_KEY", "re_chave_secreta"),
            ("RESEND_FROM", "a@b.c"),
            ("CRON_SECRET", "cron_segredo"),
        ]);
        let c = Config::from_lookup(lookup(&m)).unwrap();
        assert_eq!(c.web_url, "https://humanobill.vercel.app");
        let dbg = format!("{c:?}");
        assert!(
            !dbg.contains("re_chave_secreta") && !dbg.contains("cron_segredo"),
            "{dbg}"
        );
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
