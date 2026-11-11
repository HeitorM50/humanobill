//! Persistência das análises. `PgRepo` em produção, `MemRepo` nos testes.

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use humanobill_core::{
    Analysis, AnalysisSource, Answers, BillKind, ExtractedBill, Finding, Question,
};
use sqlx::{PgPool, Row, types::Json};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("banco: {0}")]
    Db(#[from] sqlx::Error),
    #[error("dado inválido no banco: {0}")]
    Corrupt(String),
}

/// Análise + instante de expiração (para responder 410).
#[derive(Debug, Clone)]
pub struct Stored {
    pub analysis: Analysis,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait Repo: Send + Sync {
    async fn ping(&self) -> Result<(), RepoError>;
    async fn insert(&self, a: &Analysis) -> Result<(), RepoError>;
    async fn get(&self, id: Uuid) -> Result<Option<Stored>, RepoError>;
}

pub struct PgRepo {
    pool: PgPool,
}

impl PgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>, RepoError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| RepoError::Corrupt(e.to_string()))
}

fn source_str(s: AnalysisSource) -> &'static str {
    match s {
        AnalysisSource::Upload => "upload",
        AnalysisSource::Demo => "demo",
    }
}

fn kind_str(k: BillKind) -> &'static str {
    match k {
        BillKind::Telecom => "telecom",
        BillKind::Energy => "energy",
    }
}

#[async_trait]
impl Repo for PgRepo {
    async fn ping(&self) -> Result<(), RepoError> {
        sqlx::query("select 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn insert(&self, a: &Analysis) -> Result<(), RepoError> {
        let id = Uuid::parse_str(&a.id).map_err(|e| RepoError::Corrupt(e.to_string()))?;
        sqlx::query(
            "insert into analyses (id, created_at, expires_at, source, kind, bill, answers, paid, findings, questions, recoverable_cents, rules_version)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(parse_ts(&a.created_at)?)
        .bind(parse_ts(&a.expires_at)?)
        .bind(source_str(a.source))
        .bind(kind_str(a.bill.kind))
        .bind(Json(&a.bill))
        .bind(Json(&a.answers))
        .bind(a.paid)
        .bind(Json(&a.findings))
        .bind(Json(&a.questions))
        .bind(a.recoverable_cents)
        .bind(&a.rules_version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<Stored>, RepoError> {
        let row = sqlx::query(
            "select id, created_at, expires_at, source, bill, answers, paid, findings, questions, recoverable_cents, rules_version
             from analyses where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(r) = row else { return Ok(None) };
        let created_at: DateTime<Utc> = r.try_get("created_at")?;
        let expires_at: DateTime<Utc> = r.try_get("expires_at")?;
        let source = match r.try_get::<String, _>("source")?.as_str() {
            "upload" => AnalysisSource::Upload,
            _ => AnalysisSource::Demo,
        };
        let analysis = Analysis {
            id: r.try_get::<Uuid, _>("id")?.to_string(),
            created_at: created_at.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            source,
            bill: r.try_get::<Json<ExtractedBill>, _>("bill")?.0,
            answers: r.try_get::<Json<Answers>, _>("answers")?.0,
            paid: r.try_get("paid")?,
            findings: r.try_get::<Json<Vec<Finding>>, _>("findings")?.0,
            questions: r.try_get::<Json<Vec<Question>>, _>("questions")?.0,
            recoverable_cents: r.try_get("recoverable_cents")?,
            rules_version: r.try_get("rules_version")?,
        };
        Ok(Some(Stored {
            analysis,
            expires_at,
        }))
    }
}

/// Repositório em memória (testes).
#[cfg(test)]
#[derive(Default)]
pub struct MemRepo {
    rows: Mutex<HashMap<Uuid, Analysis>>,
}

#[cfg(test)]
#[async_trait]
impl Repo for MemRepo {
    async fn ping(&self) -> Result<(), RepoError> {
        Ok(())
    }

    async fn insert(&self, a: &Analysis) -> Result<(), RepoError> {
        let id = Uuid::parse_str(&a.id).map_err(|e| RepoError::Corrupt(e.to_string()))?;
        self.rows.lock().expect("lock").insert(id, a.clone());
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<Stored>, RepoError> {
        let Some(a) = self.rows.lock().expect("lock").get(&id).cloned() else {
            return Ok(None);
        };
        let expires_at = parse_ts(&a.expires_at)?;
        Ok(Some(Stored {
            analysis: a,
            expires_at,
        }))
    }
}
