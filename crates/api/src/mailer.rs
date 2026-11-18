//! Envio de e-mail (RF18). `ResendMailer` em produção, `FakeMailer` nos testes.
//!
//! Privacidade: o endereço do usuário nunca entra em log nem em mensagem de erro.
//! Por isso `Message` e os erros têm `Debug`/`Display` sem o destinatário.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

/// Um e-mail de texto simples (mais acessível e sem rastreador).
#[derive(Clone, PartialEq, Eq)]
pub struct Message {
    pub to: String,
    pub subject: String,
    pub text: String,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("to", &"<oculto>")
            .field("subject", &self.subject)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum MailError {
    #[error("serviço de e-mail indisponível")]
    Unavailable,
    #[error("serviço de e-mail recusou o envio (status {0})")]
    Rejected(u16),
}

impl MailError {
    /// Recusa que não adianta repetir (4xx, exceto 408 e 429): endereço inválido,
    /// remetente não autorizado etc. Rede, timeout, 408, 429 e 5xx são transitórios.
    /// Só olha o código de status: nunca o endereço nem o corpo da resposta.
    pub fn is_permanent(&self) -> bool {
        matches!(self, MailError::Rejected(s) if (400..500).contains(s) && !matches!(s, 408 | 429))
    }
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, msg: &Message) -> Result<(), MailError>;
}

/// Configuração do Resend. Sem domínio próprio o Resend só entrega ao e-mail
/// da conta (P11): por isso o remetente vem do ambiente e nada é presumido.
#[derive(Clone)]
pub struct ResendConfig {
    pub api_key: String,
    /// Ex.: `HumanoBill <onboarding@resend.dev>` (env `RESEND_FROM`).
    pub from: String,
    pub base_url: String,
}

impl ResendConfig {
    pub const DEFAULT_BASE_URL: &'static str = "https://api.resend.com";
}

impl std::fmt::Debug for ResendConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResendConfig")
            .field("api_key", &"<oculta>")
            .field("from", &self.from)
            .field("base_url", &self.base_url)
            .finish()
    }
}

pub struct ResendMailer {
    client: reqwest::Client,
    cfg: ResendConfig,
}

impl ResendMailer {
    pub fn new(cfg: ResendConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("cliente HTTP válido");
        Self { client, cfg }
    }
}

#[async_trait]
impl Mailer for ResendMailer {
    async fn send(&self, msg: &Message) -> Result<(), MailError> {
        let url = format!("{}/emails", self.cfg.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.cfg.api_key)
            .json(&json!({
                "from": self.cfg.from,
                "to": [msg.to],
                "subject": msg.subject,
                "text": msg.text,
            }))
            .send()
            .await
            // O erro do reqwest pode trazer a URL e detalhes: descarta tudo.
            .map_err(|_| MailError::Unavailable)?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            // O corpo da resposta pode ecoar o destinatário: nunca é lido.
            Err(MailError::Rejected(status.as_u16()))
        }
    }
}

/// Registra os envios em memória. Nenhum e-mail sai de verdade.
#[cfg(test)]
#[derive(Default)]
pub struct FakeMailer {
    sent: std::sync::Mutex<Vec<Message>>,
    fail: std::sync::Mutex<Option<MailError>>,
}

#[cfg(test)]
impl FakeMailer {
    pub fn failing() -> Self {
        Self::failing_with(MailError::Unavailable)
    }

    pub fn failing_with(e: MailError) -> Self {
        let m = Self::default();
        *m.fail.lock().expect("lock") = Some(e);
        m
    }

    pub fn sent(&self) -> Vec<Message> {
        self.sent.lock().expect("lock").clone()
    }
}

#[cfg(test)]
#[async_trait]
impl Mailer for FakeMailer {
    async fn send(&self, msg: &Message) -> Result<(), MailError> {
        if let Some(e) = self.fail.lock().expect("lock").clone() {
            return Err(e);
        }
        self.sent.lock().expect("lock").push(msg.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    /// Servidor HTTP mínimo: responde com `status` e devolve o pedido recebido.
    async fn servidor(
        status: u16,
        corpo: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let h = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = sock.read(&mut chunk).await.unwrap();
                buf.extend_from_slice(&chunk[..n]);
                let txt = String::from_utf8_lossy(&buf).to_string();
                if let Some(i) = txt.find("\r\n\r\n") {
                    let len = txt[..i]
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if buf.len() >= i + 4 + len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let resp = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{corpo}",
                corpo.len()
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&buf).to_string()
        });
        (url, h)
    }

    fn msg() -> Message {
        Message {
            to: "maria@exemplo.com".into(),
            subject: "Assunto".into(),
            text: "Texto".into(),
        }
    }

    fn cfg(base_url: String) -> ResendConfig {
        ResendConfig {
            api_key: "re_chave_secreta".into(),
            from: "HumanoBill <onboarding@resend.dev>".into(),
            base_url,
        }
    }

    #[tokio::test]
    async fn resend_envia_post_em_emails_com_chave_e_remetente_do_ambiente() {
        let (url, pedido) = servidor(200, r#"{"id":"abc"}"#).await;
        ResendMailer::new(cfg(url)).send(&msg()).await.unwrap();
        let req = pedido.await.unwrap();
        assert!(req.starts_with("POST /emails "), "{req}");
        assert!(
            req.to_lowercase()
                .contains("authorization: bearer re_chave_secreta")
        );
        assert!(req.contains("onboarding@resend.dev"));
        assert!(req.contains("maria@exemplo.com"));
    }

    #[tokio::test]
    async fn resend_recusado_nao_vaza_destinatario_nem_corpo_da_resposta() {
        let (url, _) = servidor(403, r#"{"message":"maria@exemplo.com não verificado"}"#).await;
        let e = ResendMailer::new(cfg(url)).send(&msg()).await.unwrap_err();
        assert_eq!(e, MailError::Rejected(403));
        let txt = format!("{e} {e:?}");
        assert!(!txt.contains("maria"), "{txt}");
    }

    #[tokio::test]
    async fn resend_fora_do_ar_vira_indisponivel_sem_detalhes() {
        // Porta fechada: ninguém escuta.
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        drop(l);
        let e = ResendMailer::new(cfg(url)).send(&msg()).await.unwrap_err();
        assert_eq!(e, MailError::Unavailable);
    }

    #[test]
    fn recusa_e_permanente_so_para_4xx_exceto_408_e_429() {
        for status in [400, 401, 403, 404, 422] {
            assert!(MailError::Rejected(status).is_permanent(), "{status}");
        }
        for status in [408, 429, 500, 502, 503] {
            assert!(!MailError::Rejected(status).is_permanent(), "{status}");
        }
        assert!(!MailError::Unavailable.is_permanent());
    }

    #[test]
    fn debug_nao_mostra_destinatario_nem_chave() {
        let m = format!("{:?}", msg());
        assert!(!m.contains("maria"), "{m}");
        let c = format!("{:?}", cfg("http://x".into()));
        assert!(!c.contains("re_chave_secreta"), "{c}");
    }
}
