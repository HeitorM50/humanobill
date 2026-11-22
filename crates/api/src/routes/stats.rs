//! #62: painel coletivo anonimizado (`GET /stats`). Só lê, sem IA e sem escrita.
//!
//! Privacidade: conta só análises enviadas (`source = 'upload'`) e não expiradas.
//! Devolve apenas contagens por tipo de fatura e por regra: sem empresa, sem valor,
//! sem ids. K-anonimato: grupo com menos de `STATS_MIN_GROUP` análises não aparece.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::State,
    http::{HeaderValue, header},
    response::IntoResponse,
    routing::{MethodRouter, get},
};
use chrono::Utc;
use humanobill_core::{BillKind, KindStats, RuleStats, STATS_MIN_GROUP, Stats};

use crate::{
    error::ApiError,
    repo::{Repo, StatsCounts},
};

/// Por quanto tempo o resultado fica em memória antes de consultar o banco de novo.
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

/// Último resultado calculado e o instante em que foi guardado.
type Cache = Mutex<Option<(Instant, Arc<Stats>)>>;

/// Estado só desta rota: o repositório e o cache (um por `router`, não global).
#[derive(Clone)]
struct StatsState {
    repo: Arc<dyn Repo>,
    cache: Arc<Cache>,
}

/// Rota `GET /stats`, com cache em memória de alguns minutos.
pub(super) fn route<S: Clone + Send + Sync + 'static>(repo: Arc<dyn Repo>) -> MethodRouter<S> {
    get(get_stats).with_state(StatsState {
        repo,
        cache: Arc::new(Mutex::new(None)),
    })
}

async fn get_stats(State(s): State<StatsState>) -> Result<impl IntoResponse, ApiError> {
    let cached = s
        .cache
        .lock()
        .expect("lock")
        .as_ref()
        .filter(|(at, _)| at.elapsed() < CACHE_TTL)
        .map(|(_, stats)| stats.clone());
    let stats = match cached {
        Some(stats) => stats,
        None => {
            let counts = s.repo.stats_counts(Utc::now()).await?;
            let stats = Arc::new(build_stats(counts));
            *s.cache.lock().expect("lock") = Some((Instant::now(), stats.clone()));
            stats
        }
    };
    Ok((
        [(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=60"),
        )],
        Json((*stats).clone()),
    ))
}

/// Título curto de cada regra, em português claro. Um teste confere que todas as
/// regras do motor têm título; o `crates/rules` fica intocado.
fn short_title(rule_id: &str) -> Option<&'static str> {
    Some(match rule_id {
        "TEL-01" => "Serviço extra que você não pediu",
        "TEL-02" => "Cobrança repetida",
        "TEL-03" => "Multa de atraso acima de 2%",
        "TEL-04" => "Juros de atraso acima de 1% ao mês",
        "TEL-05" => "Mensalidade acima do plano",
        "TEL-06" => "Cobrança depois do cancelamento",
        "TEL-07" => "Soma dos itens não bate com o total",
        "TEL-08" => "Item sem descrição clara",
        "ENE-01" => "Multa ou juros de atraso acima do limite",
        "ENE-02" => "Consumo muito acima do seu normal",
        "ENE-03" => "Leitura estimada, não medida",
        "ENE-04" => "Dias de faturamento fora do padrão",
        "ENE-05" => "Bandeira tarifária diferente da vigente",
        "ENE-07" => "Seguro ou doação na conta de luz",
        "ENE-08" => "Soma dos itens não bate com o total",
        _ => return None,
    })
}

/// Aplica o k-anonimato e calcula os percentuais. Função pura (sem banco, sem relógio).
pub(super) fn build_stats(counts: StatsCounts) -> Stats {
    let total_of = |kind: BillKind| {
        counts
            .kinds
            .iter()
            .filter(|(k, _)| *k == kind)
            .map(|(_, n)| *n)
            .sum::<u32>()
    };

    let kinds = [BillKind::Telecom, BillKind::Energy]
        .into_iter()
        .map(|kind| {
            let total = total_of(kind);
            KindStats {
                kind,
                total: (total >= STATS_MIN_GROUP).then_some(total),
                has_hidden_findings: counts
                    .rules
                    .iter()
                    .any(|r| r.kind == kind && r.count < STATS_MIN_GROUP),
            }
        })
        .collect();

    let mut rules: Vec<RuleStats> = counts
        .rules
        .iter()
        .filter(|r| r.count >= STATS_MIN_GROUP)
        .filter_map(|r| {
            let total = total_of(r.kind);
            // Sem total do tipo (ou total menor que o achado) o dado é inconsistente: não mostra.
            if total < STATS_MIN_GROUP || r.count > total {
                return None;
            }
            let percent = (u64::from(r.count) * 100 + u64::from(total) / 2) / u64::from(total);
            Some(RuleStats {
                rule_id: r.rule_id.clone(),
                title: short_title(&r.rule_id)
                    .unwrap_or(r.rule_id.as_str())
                    .to_string(),
                kind: r.kind,
                severity: r.severity,
                count: r.count,
                percent: u8::try_from(percent).unwrap_or(100),
            })
        })
        .collect();
    rules.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| a.severity.cmp(&b.severity))
    });

    Stats {
        min_group: STATS_MIN_GROUP,
        kinds,
        rules,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::{DateTime, Duration as ChronoDuration};
    use http_body_util::BodyExt;
    use humanobill_core::{Analysis, AnalysisSource, Cents, Finding, Severity, validate_bill};
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        repo::{MemRepo, RepoError, RuleCount, Stored},
        routes::{AppState, new_analysis, router},
    };

    fn fixtures() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
    }

    fn bill(nome: &str) -> humanobill_core::ExtractedBill {
        let raw =
            std::fs::read_to_string(fixtures().join(format!("extracted/{nome}.json"))).unwrap();
        let b: humanobill_core::ExtractedBill = serde_json::from_str(&raw).unwrap();
        assert!(validate_bill(&b).is_empty());
        b
    }

    /// Análise com um achado artificial da regra e severidade pedidas (só para contar).
    fn analise(
        nome: &str,
        source: AnalysisSource,
        rule_id: &str,
        severity: Severity,
        expira_em: ChronoDuration,
    ) -> Analysis {
        let mut a = new_analysis(bill(nome), source);
        a.findings = vec![Finding {
            id: format!("{rule_id}:i1"),
            rule_id: rule_id.into(),
            severity,
            title: "x".into(),
            explanation: "x".into(),
            item_refs: vec!["i1".into()],
            original_text: None,
            page: None,
            amount_cents: 1234 as Cents,
            legal_basis: vec![],
        }];
        a.expires_at = (Utc::now() + expira_em).to_rfc3339();
        a
    }

    async fn inserir(repo: &MemRepo, n: usize, f: impl Fn() -> Analysis) {
        for _ in 0..n {
            repo.insert(&f()).await.unwrap();
        }
    }

    fn app_com(repo: Arc<dyn Repo>) -> Router {
        router(AppState {
            repo,
            extractor: None,
            fixtures_dir: fixtures(),
            limits: None,
        })
    }

    async fn get_stats(app: Router) -> (StatusCode, Value) {
        let resp = app
            .oneshot(Request::get("/stats").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    fn upload_valida(nome: &str, rule: &str) -> Analysis {
        analise(
            nome,
            AnalysisSource::Upload,
            rule,
            Severity::Suspicious,
            ChronoDuration::days(10),
        )
    }

    fn regra<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
        v["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule_id"] == id)
    }

    fn kind<'a>(v: &'a Value, k: &str) -> &'a Value {
        v["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["kind"] == k)
            .unwrap()
    }

    #[tokio::test]
    async fn sem_dados_devolve_vazio_e_oculto() {
        let repo = Arc::new(MemRepo::default());
        let (st, v) = get_stats(app_com(repo)).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v["min_group"], 5);
        assert!(v["rules"].as_array().unwrap().is_empty());
        assert!(kind(&v, "telecom")["total"].is_null());
        assert!(kind(&v, "energy")["total"].is_null());
    }

    // k-anonimato: 4 casos ficam ocultos; com 5 o grupo aparece.
    #[tokio::test]
    async fn limiar_k_quatro_oculta_e_cinco_aparece() {
        let repo = Arc::new(MemRepo::default());
        inserir(&repo, 4, || upload_valida("telecom-1", "TEL-01")).await;
        let (_, v) = get_stats(app_com(repo.clone())).await;
        assert!(regra(&v, "TEL-01").is_none(), "4 casos não podem aparecer");
        assert!(kind(&v, "telecom")["total"].is_null());
        assert_eq!(kind(&v, "telecom")["has_hidden_findings"], true);

        repo.insert(&upload_valida("telecom-1", "TEL-01"))
            .await
            .unwrap();
        let (_, v) = get_stats(app_com(repo)).await;
        let r = regra(&v, "TEL-01").expect("5 casos aparecem");
        assert_eq!(r["count"], 5);
        assert_eq!(r["percent"], 100);
        assert_eq!(r["severity"], "suspeito");
        assert_eq!(r["title"], "Serviço extra que você não pediu");
        assert_eq!(kind(&v, "telecom")["total"], 5);
        assert_eq!(kind(&v, "telecom")["has_hidden_findings"], false);
    }

    // Um grupo grande aparece, e um pequeno do mesmo tipo continua oculto.
    #[tokio::test]
    async fn grupo_pequeno_some_mesmo_com_total_do_tipo_visivel() {
        let repo = Arc::new(MemRepo::default());
        inserir(&repo, 6, || upload_valida("telecom-1", "TEL-01")).await;
        inserir(&repo, 4, || upload_valida("telecom-1", "TEL-02")).await;
        let (_, v) = get_stats(app_com(repo)).await;
        assert_eq!(kind(&v, "telecom")["total"], 10);
        assert_eq!(kind(&v, "telecom")["has_hidden_findings"], true);
        assert_eq!(regra(&v, "TEL-01").unwrap()["percent"], 60);
        assert!(regra(&v, "TEL-02").is_none());
    }

    #[tokio::test]
    async fn demo_nao_entra() {
        let repo = Arc::new(MemRepo::default());
        inserir(&repo, 9, || {
            analise(
                "telecom-1",
                AnalysisSource::Demo,
                "TEL-01",
                Severity::Suspicious,
                ChronoDuration::days(10),
            )
        })
        .await;
        inserir(&repo, 4, || upload_valida("telecom-1", "TEL-01")).await;
        let (_, v) = get_stats(app_com(repo)).await;
        assert!(regra(&v, "TEL-01").is_none(), "só 4 uploads: oculto");
        assert!(
            kind(&v, "telecom")["total"].is_null(),
            "demo não soma no total"
        );
    }

    #[tokio::test]
    async fn expirada_nao_entra() {
        let repo = Arc::new(MemRepo::default());
        inserir(&repo, 9, || {
            analise(
                "telecom-1",
                AnalysisSource::Upload,
                "TEL-01",
                Severity::Suspicious,
                ChronoDuration::days(-1),
            )
        })
        .await;
        inserir(&repo, 4, || upload_valida("telecom-1", "TEL-01")).await;
        let (_, v) = get_stats(app_com(repo)).await;
        assert!(regra(&v, "TEL-01").is_none(), "só 4 vigentes: oculto");
        assert!(kind(&v, "telecom")["total"].is_null());
    }

    #[tokio::test]
    async fn achado_ok_nao_conta_e_percentual_usa_o_total_do_tipo() {
        let repo = Arc::new(MemRepo::default());
        inserir(&repo, 5, || upload_valida("telecom-1", "TEL-03")).await;
        inserir(&repo, 5, || {
            analise(
                "telecom-1",
                AnalysisSource::Upload,
                "TEL-03",
                Severity::Ok,
                ChronoDuration::days(10),
            )
        })
        .await;
        // Energia não entra na conta do percentual de telecom.
        inserir(&repo, 5, || upload_valida("energia-1", "ENE-03")).await;
        let (_, v) = get_stats(app_com(repo)).await;
        let r = regra(&v, "TEL-03").unwrap();
        assert_eq!(r["count"], 5);
        assert_eq!(r["percent"], 50);
        assert_eq!(regra(&v, "ENE-03").unwrap()["percent"], 100);
        assert_eq!(kind(&v, "energy")["total"], 5);
    }

    // Mesma regra em dois itens da mesma análise conta uma vez só.
    #[tokio::test]
    async fn analise_conta_uma_vez_por_regra() {
        let repo = Arc::new(MemRepo::default());
        inserir(&repo, 5, || {
            let mut a = upload_valida("telecom-1", "TEL-01");
            let mut outro = a.findings[0].clone();
            outro.id = "TEL-01:i2".into();
            a.findings.push(outro);
            a
        })
        .await;
        let (_, v) = get_stats(app_com(repo)).await;
        assert_eq!(regra(&v, "TEL-01").unwrap()["count"], 5);
    }

    // A resposta nunca traz empresa, valor, id de análise ou texto da fatura.
    #[tokio::test]
    async fn resposta_nao_vaza_dado_individual() {
        let repo = Arc::new(MemRepo::default());
        let a = upload_valida("telecom-1", "TEL-01");
        let provider = a.bill.provider_name.clone();
        let id = a.id.clone();
        repo.insert(&a).await.unwrap();
        inserir(&repo, 5, || upload_valida("telecom-1", "TEL-01")).await;
        let resp = app_com(repo)
            .oneshot(Request::get("/stats").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let texto = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(!texto.contains(&provider));
        assert!(!texto.contains(&id));
        assert!(!texto.contains("1234") && !texto.contains("cents"));
        let v: Value = serde_json::from_str(&texto).unwrap();
        for r in v["rules"].as_array().unwrap() {
            let mut chaves: Vec<_> = r.as_object().unwrap().keys().cloned().collect();
            chaves.sort();
            assert_eq!(
                chaves,
                ["count", "kind", "percent", "rule_id", "severity", "title"]
            );
        }
    }

    #[test]
    fn toda_regra_do_motor_tem_titulo_curto() {
        for r in humanobill_rules::RULES {
            assert!(short_title(r.id).is_some(), "{} sem título curto", r.id);
        }
    }

    #[test]
    fn ordena_do_mais_frequente_para_o_menos() {
        let c = |rule: &str, n| RuleCount {
            kind: BillKind::Telecom,
            rule_id: rule.into(),
            severity: Severity::Suspicious,
            count: n,
        };
        let s = build_stats(StatsCounts {
            kinds: vec![(BillKind::Telecom, 20)],
            rules: vec![c("TEL-02", 6), c("TEL-01", 12), c("TEL-03", 6)],
        });
        let ids: Vec<_> = s.rules.iter().map(|r| r.rule_id.as_str()).collect();
        assert_eq!(ids, ["TEL-01", "TEL-02", "TEL-03"]);
    }

    /// Repositório que conta quantas vezes o banco foi consultado.
    struct ContaConsultas {
        inner: MemRepo,
        consultas: AtomicUsize,
    }

    #[async_trait]
    impl Repo for ContaConsultas {
        async fn ping(&self) -> Result<(), RepoError> {
            Ok(())
        }
        async fn insert(&self, a: &Analysis) -> Result<(), RepoError> {
            self.inner.insert(a).await
        }
        async fn get(&self, id: uuid::Uuid) -> Result<Option<Stored>, RepoError> {
            self.inner.get(id).await
        }
        async fn update(&self, a: &Analysis) -> Result<(), RepoError> {
            self.inner.update(a).await
        }
        async fn stats_counts(&self, now: DateTime<Utc>) -> Result<StatsCounts, RepoError> {
            self.consultas.fetch_add(1, Ordering::SeqCst);
            self.inner.stats_counts(now).await
        }
    }

    #[tokio::test]
    async fn cache_evita_consultar_o_banco_de_novo() {
        let repo = Arc::new(ContaConsultas {
            inner: MemRepo::default(),
            consultas: AtomicUsize::new(0),
        });
        let app = app_com(repo.clone());
        for _ in 0..3 {
            let (st, _) = get_stats(app.clone()).await;
            assert_eq!(st, StatusCode::OK);
        }
        assert_eq!(repo.consultas.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn banco_fora_do_ar_e_500_sem_vazar_detalhe() {
        struct Quebrado;
        #[async_trait]
        impl Repo for Quebrado {
            async fn ping(&self) -> Result<(), RepoError> {
                Ok(())
            }
            async fn insert(&self, _: &Analysis) -> Result<(), RepoError> {
                Ok(())
            }
            async fn get(&self, _: uuid::Uuid) -> Result<Option<Stored>, RepoError> {
                Ok(None)
            }
            async fn update(&self, _: &Analysis) -> Result<(), RepoError> {
                Ok(())
            }
            async fn stats_counts(&self, _: DateTime<Utc>) -> Result<StatsCounts, RepoError> {
                Err(RepoError::Db(sqlx::Error::PoolTimedOut))
            }
        }
        let (st, v) = get_stats(app_com(Arc::new(Quebrado))).await;
        assert!(st.is_server_error());
        assert!(v["error"]["code"].is_string());
    }
}
