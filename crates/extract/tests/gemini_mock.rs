//! Testes de borda de `GeminiExtractor::extract` contra um servidor HTTP falso
//! em 127.0.0.1 (só `std::net`; nenhuma chamada ao Gemini real).
//!
//! Cobre: erros HTTP (429/404/5xx/4xx), fallback de modelo, retry com backoff,
//! resposta sem texto, JSON inválido na resposta, o retry de validação
//! (uma única nova tentativa) e erros de conexão.
//!
//! Se `EXTRACT_CACHE_DIR` estiver definida no ambiente, o cache de respostas
//! brutas mascararia as chamadas HTTP; nesse caso os testes são pulados.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use humanobill_core::ValidationIssue;
use humanobill_extract::{BillExtractor, ExtractError, GeminiConfig, GeminiExtractor, InputMime};
use serde_json::{Value, json};

const FATURA_OK: &str = r#"{"kind":"telecom","provider_name":"Vivaz Telecom","reference_month":"2026-10",
  "due_date":"2026-11-10","total_cents":9990,"items":[{"id":"i1","description":"Plano",
  "amount_cents":9990,"category":"plano","original_text":"PLANO 99,90","page":1,
  "plain_explanation":"Mensalidade do plano."}],"extraction_confidence":0.9}"#;

/// Resposta programada do servidor falso.
#[derive(Clone)]
struct Resp {
    status: u16,
    corpo: String,
    /// Fecha a conexão sem responder nada.
    derrubar: bool,
}

fn http(status: u16, corpo: &str) -> Resp {
    Resp {
        status,
        corpo: corpo.to_owned(),
        derrubar: false,
    }
}

/// 200 com o texto do modelo dentro de `candidates[0].content.parts[0].text`.
fn ok_texto(texto: &str) -> Resp {
    http(
        200,
        &json!({"candidates": [{"content": {"parts": [{"text": texto}]}}]}).to_string(),
    )
}

fn erro_google(status: u16, msg: &str) -> Resp {
    http(
        status,
        &json!({"error": {"code": status, "message": msg}}).to_string(),
    )
}

#[derive(Debug, Clone)]
struct Requisicao {
    caminho: String,
    chave_api: Option<String>,
    corpo: Value,
}

struct Mock {
    base_url: String,
    vistas: Arc<Mutex<Vec<Requisicao>>>,
}

impl Mock {
    /// Sobe o servidor. Cada requisição consome a próxima resposta; a última se repete.
    /// Devolve `None` se o cache de respostas estiver ligado no ambiente.
    fn novo(script: Vec<Resp>) -> Option<Mock> {
        if std::env::var_os("EXTRACT_CACHE_DIR").is_some() {
            eprintln!("EXTRACT_CACHE_DIR definida: teste com mock pulado");
            return None;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let porta = listener.local_addr().unwrap().port();
        let vistas = Arc::new(Mutex::new(Vec::new()));
        let fila = Arc::new(Mutex::new(VecDeque::from(script)));
        let vistas2 = vistas.clone();
        thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(conn) = conn else { continue };
                let resp = {
                    let mut f = fila.lock().unwrap();
                    if f.len() > 1 {
                        f.pop_front().unwrap()
                    } else {
                        f.front().cloned().unwrap_or_else(|| http(500, ""))
                    }
                };
                // Uma conexão por requisição (o cliente recebe `Connection: close`).
                let vistas = vistas2.clone();
                thread::spawn(move || atender(conn, &resp, &vistas));
            }
        });
        Some(Mock {
            base_url: format!("http://127.0.0.1:{porta}"),
            vistas,
        })
    }

    fn requisicoes(&self) -> Vec<Requisicao> {
        self.vistas.lock().unwrap().clone()
    }

    fn extrator(&self, modelo: &str, fallback: Option<&str>) -> GeminiExtractor {
        GeminiExtractor::new(GeminiConfig {
            api_key: "chave-sintetica-123".into(),
            model: modelo.into(),
            fallback_model: fallback.map(str::to_owned),
            base_url: self.base_url.clone(),
        })
    }
}

fn atender(mut conn: TcpStream, resp: &Resp, vistas: &Arc<Mutex<Vec<Requisicao>>>) {
    conn.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let (fim_cab, tam_corpo) = loop {
        let n = conn.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let cab = String::from_utf8_lossy(&buf[..i]).to_lowercase();
            let tam = cab
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            break (i + 4, tam);
        }
    };
    while buf.len() < fim_cab + tam_corpo {
        let n = conn.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let cab = String::from_utf8_lossy(&buf[..fim_cab]).into_owned();
    let caminho = cab
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("")
        .to_owned();
    let chave_api = cab.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.eq_ignore_ascii_case("x-goog-api-key")
            .then(|| v.trim().to_owned())
    });
    let corpo = serde_json::from_slice(&buf[fim_cab..]).unwrap_or(Value::Null);
    vistas.lock().unwrap().push(Requisicao {
        caminho,
        chave_api,
        corpo,
    });

    if resp.derrubar {
        return; // fecha sem responder
    }
    let texto = format!(
        "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        resp.status,
        resp.corpo.len(),
        resp.corpo
    );
    let _ = conn.write_all(texto.as_bytes());
    let _ = conn.flush();
}

fn caminho_do_modelo(m: &str) -> String {
    format!("/models/{m}:generateContent")
}

fn texto_da_nota(r: &Requisicao) -> Option<String> {
    r.corpo
        .pointer("/contents/0/parts/2/text")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

// ---------- caminho feliz e cabeçalhos ----------

#[tokio::test]
async fn sucesso_envia_chave_no_cabecalho_e_nao_na_url() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let bill = ex.extract(b"%PDF-1.7", InputMime::Pdf).await.unwrap();
    assert_eq!(bill.total_cents, 9990);
    assert_eq!(ex.http_calls(), 1);
    let reqs = m.requisicoes();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].caminho, caminho_do_modelo("m1"));
    assert_eq!(reqs[0].chave_api.as_deref(), Some("chave-sintetica-123"));
    assert!(
        !reqs[0].caminho.contains("chave-sintetica-123"),
        "chave na URL"
    );
    assert!(!reqs[0].corpo.to_string().contains("chave-sintetica-123"));
}

#[tokio::test]
async fn resposta_com_cerca_de_codigo_e_aceita() {
    let Some(m) = Mock::novo(vec![ok_texto(&format!("```json\n{FATURA_OK}\n```"))]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    assert!(ex.extract(b"x", InputMime::Png).await.is_ok());
    assert_eq!(ex.http_calls(), 1);
}

#[tokio::test]
async fn arquivo_vazio_nao_causa_panic_e_e_enviado_com_base64_vazio() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let _ = ex.extract(&[], InputMime::Pdf).await;
    let reqs = m.requisicoes();
    assert_eq!(
        reqs[0]
            .corpo
            .pointer("/contents/0/parts/0/inline_data/data")
            .unwrap(),
        ""
    );
}

#[tokio::test]
async fn arquivo_grande_de_varios_megabytes_e_enviado_inteiro() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let grande = vec![0xABu8; 3 * 1024 * 1024 + 1];
    assert!(ex.extract(&grande, InputMime::Pdf).await.is_ok());
    let reqs = m.requisicoes();
    let b64 = reqs[0]
        .corpo
        .pointer("/contents/0/parts/0/inline_data/data")
        .unwrap();
    assert_eq!(b64.as_str().unwrap().len(), grande.len().div_ceil(3) * 4);
}

// ---------- erros HTTP ----------

#[tokio::test]
async fn http_429_em_todos_os_modelos_vira_rate_limited_sem_repetir_nenhum() {
    let Some(m) = Mock::novo(vec![erro_google(429, "quota")]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2,m3"));
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(matches!(err, ExtractError::RateLimited), "{err:?}");
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(
        caminhos,
        [
            caminho_do_modelo("m1"),
            caminho_do_modelo("m2"),
            caminho_do_modelo("m3")
        ],
        "cada modelo é tentado uma vez só: sem quota não adianta insistir"
    );
}

#[tokio::test]
async fn http_429_no_principal_passa_para_o_fallback_que_tem_quota() {
    let Some(m) = Mock::novo(vec![erro_google(429, "quota"), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(caminhos, [caminho_do_modelo("m1"), caminho_do_modelo("m2")]);
}

#[tokio::test]
async fn http_400_e_fatal_com_a_mensagem_do_google() {
    let Some(m) = Mock::novo(vec![erro_google(400, "API key not valid. çãõ 🚀")]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    match err {
        ExtractError::Provider(msg) => {
            assert!(msg.contains("400"), "{msg}");
            assert!(msg.contains("(m1)"), "{msg}");
            assert!(msg.contains("API key not valid. çãõ 🚀"), "{msg}");
        }
        outro => panic!("esperava Provider, veio {outro:?}"),
    }
    assert_eq!(
        ex.http_calls(),
        1,
        "4xx (exceto 404/429) não tenta fallback"
    );
}

#[tokio::test]
async fn demais_status_de_cliente_e_estranhos_sao_fatais() {
    for status in [401u16, 403, 408, 413, 418, 422] {
        let Some(m) = Mock::novo(vec![erro_google(status, "nope")]) else {
            return;
        };
        let ex = m.extrator("m1", Some("m2"));
        let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
        assert!(
            matches!(err, ExtractError::Provider(_)),
            "status {status}: {err:?}"
        );
        assert_eq!(ex.http_calls(), 1, "status {status}");
    }
}

#[tokio::test]
async fn erro_sem_corpo_json_gera_mensagem_com_detalhe_vazio() {
    let Some(m) = Mock::novo(vec![http(400, "<html>Bad Request</html>")]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    match err {
        ExtractError::Provider(msg) => {
            assert!(msg.starts_with("HTTP 400"), "{msg}");
            assert!(!msg.contains("<html>"), "corpo bruto não deve vazar: {msg}");
        }
        outro => panic!("{outro:?}"),
    }
}

#[tokio::test]
async fn erro_json_sem_campo_message_tem_detalhe_vazio() {
    let Some(m) = Mock::novo(vec![http(400, r#"{"error":{"code":400}}"#)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(
        matches!(&err, ExtractError::Provider(s) if s.starts_with("HTTP 400") && s.ends_with(": "))
    );
}

// ---------- fallback de modelo ----------

#[tokio::test]
async fn http_404_no_principal_pula_direto_para_o_fallback() {
    let Some(m) = Mock::novo(vec![
        erro_google(404, "model not found"),
        ok_texto(FATURA_OK),
    ]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    let reqs = m.requisicoes();
    assert_eq!(reqs.len(), 2, "404 não deve repetir o mesmo modelo");
    assert_eq!(reqs[0].caminho, caminho_do_modelo("m1"));
    assert_eq!(reqs[1].caminho, caminho_do_modelo("m2"));
}

#[tokio::test]
async fn http_404_em_todos_os_modelos_devolve_erro_do_ultimo() {
    let Some(m) = Mock::novo(vec![erro_google(404, "sumiu")]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2, m3"));
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    match err {
        ExtractError::Provider(msg) => {
            assert!(msg.contains("(m3)") && msg.contains("404"), "{msg}")
        }
        outro => panic!("{outro:?}"),
    }
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(
        caminhos,
        [
            caminho_do_modelo("m1"),
            caminho_do_modelo("m2"),
            caminho_do_modelo("m3")
        ]
    );
}

#[tokio::test]
async fn fallback_vazio_ou_so_com_virgulas_equivale_a_nao_ter_fallback() {
    let Some(m) = Mock::novo(vec![erro_google(404, "sumiu")]) else {
        return;
    };
    let ex = m.extrator("m1", Some(" , ,, "));
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_err());
    assert_eq!(m.requisicoes().len(), 1);
}

#[tokio::test]
async fn http_404_no_principal_e_429_nos_fallbacks_e_rate_limited() {
    let Some(m) = Mock::novo(vec![erro_google(404, "sumiu"), erro_google(429, "quota")]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2,m3"));
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(matches!(err, ExtractError::RateLimited), "{err:?}");
    assert_eq!(ex.http_calls(), 3, "m1 404, m2 429, m3 429");
}

// ---------- erros transitórios (5xx) e backoff ----------

#[tokio::test]
async fn http_503_uma_vez_e_depois_sucesso_repete_no_mesmo_modelo_se_nao_ha_outro() {
    let Some(m) = Mock::novo(vec![erro_google(503, "sobrecarga"), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let t = std::time::Instant::now();
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert!(
        t.elapsed() >= Duration::from_millis(900),
        "deveria esperar o backoff de 1s"
    );
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(caminhos, [caminho_do_modelo("m1"), caminho_do_modelo("m1")]);
}

#[tokio::test]
async fn http_503_no_principal_passa_logo_para_o_fallback_sem_esperar() {
    let Some(m) = Mock::novo(vec![erro_google(503, "alta demanda"), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    let t = std::time::Instant::now();
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert!(
        t.elapsed() < Duration::from_millis(900),
        "não deve esperar o backoff de 1s com outro modelo disponível"
    );
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(caminhos, [caminho_do_modelo("m1"), caminho_do_modelo("m2")]);
}

#[tokio::test]
async fn modelo_com_503_fica_de_lado_nas_proximas_extracoes() {
    let Some(m) = Mock::novo(vec![
        erro_google(503, "alta demanda"),
        ok_texto(FATURA_OK),
        ok_texto(FATURA_OK),
    ]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(
        caminhos,
        [
            caminho_do_modelo("m1"),
            caminho_do_modelo("m2"),
            caminho_do_modelo("m2")
        ],
        "a segunda extração nem tenta m1, que acabou de falhar"
    );
}

#[tokio::test]
async fn modelo_que_deu_404_nao_e_tentado_de_novo_logo_em_seguida() {
    let Some(m) = Mock::novo(vec![
        erro_google(404, "sumiu"),
        ok_texto(FATURA_OK),
        ok_texto(FATURA_OK),
    ]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
    assert_eq!(
        caminhos,
        [
            caminho_do_modelo("m1"),
            caminho_do_modelo("m2"),
            caminho_do_modelo("m2")
        ]
    );
}

#[tokio::test]
async fn se_todos_os_modelos_estao_de_lado_tenta_todos_de_novo() {
    let Some(m) = Mock::novo(vec![
        erro_google(429, "quota"),
        erro_google(429, "quota"),
        ok_texto(FATURA_OK),
    ]) else {
        return;
    };
    let ex = m.extrator("m1", Some("m2"));
    assert!(matches!(
        ex.extract(b"x", InputMime::Pdf).await.unwrap_err(),
        ExtractError::RateLimited
    ));
    // Ambos ficaram de lado, mas a quota pode ter voltado: a próxima extração tenta de novo.
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert_eq!(ex.http_calls(), 3);
}

#[tokio::test]
async fn http_5xx_esgota_tentativas_do_principal_e_passa_para_o_fallback() {
    for status in [500u16, 502, 504] {
        let Some(m) = Mock::novo(vec![
            erro_google(status, "falha"),
            erro_google(status, "falha"),
            ok_texto(FATURA_OK),
        ]) else {
            return;
        };
        let ex = m.extrator("m1", Some("m2"));
        assert!(
            ex.extract(b"x", InputMime::Pdf).await.is_ok(),
            "status {status}"
        );
        let caminhos: Vec<_> = m.requisicoes().into_iter().map(|r| r.caminho).collect();
        assert_eq!(
            caminhos,
            [
                caminho_do_modelo("m1"),
                caminho_do_modelo("m1"),
                caminho_do_modelo("m2")
            ],
            "status {status}"
        );
    }
}

#[tokio::test]
async fn http_503_sempre_sem_fallback_faz_exatamente_attempts_per_model_chamadas() {
    let Some(m) = Mock::novo(vec![erro_google(503, "sobrecarga")]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(
        matches!(&err, ExtractError::Provider(s) if s.contains("503")),
        "{err:?}"
    );
    assert_eq!(ex.http_calls(), GeminiConfig::ATTEMPTS_PER_MODEL);
}

#[tokio::test]
async fn gateway_502_com_html_nao_vaza_o_html_na_mensagem() {
    let Some(m) = Mock::novo(vec![http(
        502,
        "<html><body>Bad gateway segredo-xyz</body></html>",
    )]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("502"), "{msg}");
    assert!(!msg.contains("segredo-xyz"), "{msg}");
}

// ---------- resposta 200 estranha ----------

#[tokio::test]
async fn http_200_sem_candidatos_vira_provider_resposta_sem_texto() {
    for corpo in [
        r#"{}"#,
        r#"{"candidates":[]}"#,
        r#"{"candidates":[{"finishReason":"SAFETY"}]}"#,
        r#"{"candidates":[{"content":{"parts":[{"text":""}]}}]}"#,
    ] {
        let Some(m) = Mock::novo(vec![http(200, corpo)]) else {
            return;
        };
        let ex = m.extrator("m1", Some("m2"));
        let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
        assert!(
            matches!(&err, ExtractError::Provider(s) if s == "resposta sem texto"),
            "corpo {corpo}: {err:?}"
        );
        assert_eq!(ex.http_calls(), 1, "sem fallback para 200 vazio ({corpo})");
    }
}

#[tokio::test]
async fn http_200_com_corpo_que_nao_e_json_vira_provider() {
    for corpo in ["", "oi", "<html></html>", "{\"candidates\":"] {
        let Some(m) = Mock::novo(vec![http(200, corpo)]) else {
            return;
        };
        let ex = m.extrator("m1", None);
        let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
        assert!(
            matches!(err, ExtractError::Provider(_)),
            "corpo {corpo:?}: {err:?}"
        );
        assert_eq!(ex.http_calls(), 1);
    }
}

#[tokio::test]
async fn http_204_sem_conteudo_vira_provider() {
    let Some(m) = Mock::novo(vec![http(204, "")]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(matches!(err, ExtractError::Provider(_)), "{err:?}");
}

// ---------- retry de validação (uma única nova tentativa) ----------

#[tokio::test]
async fn json_fora_do_schema_dispara_um_retry_com_nota_generica() {
    let Some(m) = Mock::novo(vec![ok_texto("isto não é json"), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    let reqs = m.requisicoes();
    assert_eq!(reqs.len(), 2);
    assert!(
        texto_da_nota(&reqs[0]).is_none(),
        "primeira chamada não tem nota"
    );
    let nota = texto_da_nota(&reqs[1]).expect("segunda chamada tem nota");
    assert!(nota.contains("o JSON não segue o schema"), "{nota}");
}

#[tokio::test]
async fn soma_divergente_dispara_retry_com_as_issues_na_nota() {
    let errada = FATURA_OK.replace("\"total_cents\":9990", "\"total_cents\":19990");
    let Some(m) = Mock::novo(vec![ok_texto(&errada), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    let nota = texto_da_nota(&m.requisicoes()[1]).unwrap();
    assert!(nota.contains("SumMismatch"), "{nota}");
    assert!(nota.contains("19990"), "{nota}");
}

#[tokio::test]
async fn a_nota_do_retry_nao_repete_o_conteudo_da_fatura() {
    // RNF13: a nota só descreve o problema, nunca o texto extraído.
    let errada = FATURA_OK
        .replace("\"total_cents\":9990", "\"total_cents\":19990")
        .replace("Vivaz Telecom", "Marca-Secreta-ZZZ");
    let Some(m) = Mock::novo(vec![ok_texto(&errada), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let _ = ex.extract(b"x", InputMime::Pdf).await;
    let nota = texto_da_nota(&m.requisicoes()[1]).unwrap();
    assert!(
        !nota.contains("Marca-Secreta-ZZZ") && !nota.contains("PLANO 99,90"),
        "{nota}"
    );
}

#[tokio::test]
async fn duas_respostas_invalidas_param_no_segundo_retorno_com_as_issues() {
    let errada = FATURA_OK.replace("\"total_cents\":9990", "\"total_cents\":19990");
    let Some(m) = Mock::novo(vec![ok_texto(&errada)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    match err {
        ExtractError::Invalid { issues } => {
            assert!(
                matches!(issues[..], [ValidationIssue::SumMismatch { .. }]),
                "{issues:?}"
            )
        }
        outro => panic!("{outro:?}"),
    }
    assert_eq!(ex.http_calls(), 2, "só um retry, nunca um terceiro");
}

#[tokio::test]
async fn duas_respostas_que_nao_sao_json_dao_invalid_com_issues_vazias() {
    let Some(m) = Mock::novo(vec![ok_texto("lixo")]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(
        matches!(&err, ExtractError::Invalid { issues } if issues.is_empty()),
        "{err:?}"
    );
    assert_eq!(ex.http_calls(), 2);
}

#[tokio::test]
async fn erro_de_quota_no_retry_de_validacao_e_propagado() {
    let Some(m) = Mock::novo(vec![ok_texto("lixo"), erro_google(429, "quota")]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    assert!(matches!(err, ExtractError::RateLimited), "{err:?}");
    assert_eq!(ex.http_calls(), 2);
}

#[tokio::test]
async fn primeira_valida_nao_faz_segunda_chamada() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK), ok_texto("lixo")]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert_eq!(ex.http_calls(), 1);
}

#[tokio::test]
async fn varias_extracoes_acumulam_o_contador_de_chamadas() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    for _ in 0..3 {
        ex.extract(b"x", InputMime::Pdf).await.unwrap();
    }
    assert_eq!(ex.http_calls(), 3);
}

#[tokio::test]
async fn extracoes_concorrentes_nao_misturam_o_contador() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = Arc::new(m.extrator("m1", None));
    let mut tarefas = Vec::new();
    for _ in 0..8 {
        let ex = ex.clone();
        tarefas.push(tokio::spawn(async move {
            ex.extract(b"x", InputMime::Pdf).await
        }));
    }
    for t in tarefas {
        assert!(t.await.unwrap().is_ok());
    }
    assert_eq!(ex.http_calls(), 8);
}

// ---------- falhas de conexão ----------

#[tokio::test]
async fn conexao_recusada_vira_provider_sem_vazar_a_url() {
    // Reserva uma porta e fecha o listener: ninguém atende.
    let porta = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    if std::env::var_os("EXTRACT_CACHE_DIR").is_some() {
        return;
    }
    let ex = GeminiExtractor::new(GeminiConfig {
        api_key: "chave-sintetica-123".into(),
        model: "m1".into(),
        fallback_model: None,
        base_url: format!("http://127.0.0.1:{porta}"),
    });
    let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
    let msg = err.to_string();
    assert!(matches!(err, ExtractError::Provider(_)), "{msg}");
    assert!(
        !msg.contains("generateContent"),
        "URL vazou na mensagem: {msg}"
    );
    assert!(!msg.contains("chave-sintetica-123"), "{msg}");
    assert_eq!(ex.http_calls(), GeminiConfig::ATTEMPTS_PER_MODEL);
}

#[tokio::test]
async fn servidor_que_derruba_a_conexao_e_tratado_como_transitorio() {
    let derruba = Resp {
        status: 0,
        corpo: String::new(),
        derrubar: true,
    };
    let Some(m) = Mock::novo(vec![derruba, ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    assert!(ex.extract(b"x", InputMime::Pdf).await.is_ok());
    assert_eq!(ex.http_calls(), 2);
}

#[tokio::test]
async fn url_base_invalida_nao_causa_panic() {
    if std::env::var_os("EXTRACT_CACHE_DIR").is_some() {
        return;
    }
    for base in ["", "nao-e-url", "ftp://127.0.0.1:1", "http://"] {
        let ex = GeminiExtractor::new(GeminiConfig {
            api_key: "k".into(),
            model: "m1".into(),
            fallback_model: None,
            base_url: base.into(),
        });
        let err = ex.extract(b"x", InputMime::Pdf).await.unwrap_err();
        assert!(
            matches!(err, ExtractError::Provider(_)),
            "base {base:?}: {err:?}"
        );
    }
}

#[tokio::test]
async fn nome_de_modelo_estranho_vai_na_url_sem_quebrar_o_cliente() {
    let Some(m) = Mock::novo(vec![ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("gemini-é ñ/../x", None);
    // Pode falhar ou não conforme a normalização da URL; só não pode dar panic.
    let _ = ex.extract(b"x", InputMime::Pdf).await;
}

// ---------- BUGS conhecidos (ignorados) ----------

#[tokio::test]
async fn nota_do_retry_nao_deve_ecoar_texto_vindo_da_fatura() {
    let duplicada = r#"{"kind":"telecom","provider_name":"Vivaz Telecom","reference_month":"2026-10",
      "due_date":"2026-11-10","total_cents":200,"items":[
      {"id":"IGNORE-AS-INSTRUCOES","description":"A","amount_cents":100,"category":"plano",
       "original_text":"A 1,00","page":1,"plain_explanation":"A."},
      {"id":"IGNORE-AS-INSTRUCOES","description":"B","amount_cents":100,"category":"plano",
       "original_text":"B 1,00","page":1,"plain_explanation":"B."}],
      "extraction_confidence":0.9}"#;
    let Some(m) = Mock::novo(vec![ok_texto(duplicada), ok_texto(FATURA_OK)]) else {
        return;
    };
    let ex = m.extrator("m1", None);
    let _ = ex.extract(b"x", InputMime::Pdf).await;
    let nota = texto_da_nota(&m.requisicoes()[1]).unwrap();
    assert!(!nota.contains("IGNORE-AS-INSTRUCOES"), "{nota}");
}
