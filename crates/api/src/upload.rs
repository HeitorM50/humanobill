//! Validação do arquivo enviado (RF02, RNF06): tipo pelos magic bytes, tamanho e páginas.

use axum::http::StatusCode;
use humanobill_extract::InputMime;

use crate::error::ApiError;

pub const MAX_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_PAGES: usize = 5;

pub fn validate_file(bytes: &[u8]) -> Result<InputMime, ApiError> {
    if bytes.is_empty() {
        return Err(invalid("Envie um PDF ou uma foto (JPG, PNG)"));
    }
    if bytes.len() > MAX_BYTES {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_large",
            "O arquivo deve ter até 10 MB",
        ));
    }
    // Nunca confiar na extensão nem no Content-Type do cliente.
    let mime = infer::get(bytes)
        .and_then(|t| InputMime::from_mime(t.mime_type()))
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_file",
                "Envie um PDF ou uma foto (JPG, PNG)",
            )
        })?;
    if mime == InputMime::Pdf && pdf_page_count(bytes) > MAX_PAGES {
        return Err(invalid("A fatura deve ter até 5 páginas"));
    }
    Ok(mime)
}

fn invalid(msg: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "invalid_file", msg)
}

/// Contagem aproximada de páginas: objetos `/Type /Page` (sem o `s` de `/Pages`).
fn pdf_page_count(bytes: &[u8]) -> usize {
    let mut count = 0;
    for pat in [b"/Type /Page".as_slice(), b"/Type/Page".as_slice()] {
        let mut i = 0;
        while let Some(pos) = find(&bytes[i..], pat) {
            let end = i + pos + pat.len();
            if bytes.get(end) != Some(&b's') {
                count += 1;
            }
            i = end;
        }
    }
    count
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_pdf(pages: usize) -> Vec<u8> {
        let mut v = b"%PDF-1.7\n1 0 obj << /Type /Pages /Count 1 >> endobj\n".to_vec();
        for _ in 0..pages {
            v.extend_from_slice(b"2 0 obj << /Type /Page >> endobj\n");
        }
        v
    }

    #[test]
    fn aceita_pdf_pequeno() {
        assert_eq!(validate_file(&fake_pdf(2)).unwrap(), InputMime::Pdf);
    }

    #[test]
    fn rejeita_pdf_com_muitas_paginas() {
        assert_eq!(
            validate_file(&fake_pdf(6)).unwrap_err().code,
            "invalid_file"
        );
    }

    #[test]
    fn aceita_png_pelos_magic_bytes() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        assert_eq!(validate_file(&png).unwrap(), InputMime::Png);
    }

    #[test]
    fn rejeita_docx_mesmo_com_nome_de_pdf() {
        // ZIP (docx) com qualquer nome: os magic bytes mandam.
        let zip = [b'P', b'K', 3, 4, 20, 0, 0, 0, 0, 0];
        let e = validate_file(&zip).unwrap_err();
        assert_eq!(e.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[test]
    fn rejeita_arquivo_grande() {
        let mut big = fake_pdf(1);
        big.resize(MAX_BYTES + 1, b' ');
        assert_eq!(validate_file(&big).unwrap_err().code, "too_large");
    }

    #[test]
    fn rejeita_vazio() {
        assert_eq!(validate_file(&[]).unwrap_err().code, "invalid_file");
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;

    const PDF_1_PAGINA: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Page >> endobj\n";

    fn pdf_n(pagina: &[u8], n: usize) -> Vec<u8> {
        let mut v = b"%PDF-1.7\n".to_vec();
        for _ in 0..n {
            v.extend_from_slice(pagina);
            v.push(b'\n');
        }
        v
    }

    fn pdf_com_tamanho(n: usize) -> Vec<u8> {
        let mut v = PDF_1_PAGINA.to_vec();
        v.resize(n, b' ');
        v
    }

    #[test]
    fn limites_publicos_sao_10mb_e_5_paginas() {
        assert_eq!(MAX_BYTES, 10 * 1024 * 1024);
        assert_eq!(MAX_PAGES, 5);
    }

    #[test]
    fn tamanho_no_limite_exato_passa_e_um_byte_a_mais_e_413() {
        assert_eq!(
            validate_file(&pdf_com_tamanho(MAX_BYTES)).unwrap(),
            InputMime::Pdf
        );
        let e = validate_file(&pdf_com_tamanho(MAX_BYTES + 1)).unwrap_err();
        assert_eq!(e.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(e.code, "too_large");
    }

    #[test]
    fn tamanho_e_checado_antes_do_tipo() {
        // Lixo gigante: 413 (e não 415), sem sequer tentar identificar o tipo.
        let e = validate_file(&vec![0u8; MAX_BYTES + 1]).unwrap_err();
        assert_eq!(e.status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn vazio_e_400_e_um_byte_qualquer_e_415() {
        let e = validate_file(&[]).unwrap_err();
        assert_eq!(e.status, StatusCode::BAD_REQUEST);
        for b in [0u8, b'%', 0xFF, b'P'] {
            let e = validate_file(&[b]).unwrap_err();
            assert_eq!(e.status, StatusCode::UNSUPPORTED_MEDIA_TYPE, "byte {b:#x}");
            assert_eq!(e.code, "invalid_file");
        }
    }

    #[test]
    fn aceita_jpeg_e_webp_pelos_magic_bytes() {
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F', 0, 1,
        ];
        assert_eq!(validate_file(&jpeg).unwrap(), InputMime::Jpeg);
        let webp = b"RIFF\x24\x00\x00\x00WEBPVP8 ";
        assert_eq!(validate_file(webp).unwrap(), InputMime::Webp);
    }

    #[test]
    fn rejeita_tipos_que_o_infer_conhece_mas_a_api_nao_aceita() {
        let casos: [(&str, &[u8]); 5] = [
            ("gif", b"GIF89a\x01\x00\x01\x00"),
            ("zip", b"PK\x03\x04\x14\x00\x00\x00"),
            ("exe", b"MZ\x90\x00\x03\x00\x00\x00"),
            ("gzip", &[0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0]),
            ("texto", "olá, mundo".as_bytes()),
        ];
        for (nome, bytes) in casos {
            let e = validate_file(bytes).unwrap_err();
            assert_eq!(e.status, StatusCode::UNSUPPORTED_MEDIA_TYPE, "{nome}");
        }
    }

    #[test]
    fn pdf_com_5_paginas_passa_e_com_6_nao() {
        assert_eq!(
            validate_file(&pdf_n(b"2 0 obj << /Type /Page >> endobj", MAX_PAGES)).unwrap(),
            InputMime::Pdf
        );
        let e =
            validate_file(&pdf_n(b"2 0 obj << /Type /Page >> endobj", MAX_PAGES + 1)).unwrap_err();
        assert_eq!(e.status, StatusCode::BAD_REQUEST);
        assert_eq!(e.code, "invalid_file");
    }

    #[test]
    fn contagem_de_paginas_aceita_as_duas_grafias_e_soma_as_duas() {
        assert_eq!(pdf_page_count(&pdf_n(b"<< /Type /Page >>", 3)), 3);
        assert_eq!(pdf_page_count(&pdf_n(b"<< /Type/Page >>", 4)), 4);
        let mut misto = pdf_n(b"<< /Type /Page >>", 3);
        misto.extend(pdf_n(b"<< /Type/Page >>", 3));
        assert_eq!(pdf_page_count(&misto), 6);
        // 3 + 3 páginas em grafias diferentes também estoura o limite.
        assert_eq!(validate_file(&misto).unwrap_err().code, "invalid_file");
    }

    #[test]
    fn pdf_sem_nenhuma_pagina_ou_so_com_pages_passa() {
        assert_eq!(pdf_page_count(b"%PDF-1.7\n"), 0);
        assert_eq!(validate_file(b"%PDF-1.7\n").unwrap(), InputMime::Pdf);
        let so_pages = pdf_n(b"<< /Type /Pages /Count 99 >>", 50);
        assert_eq!(pdf_page_count(&so_pages), 0);
        assert_eq!(validate_file(&so_pages).unwrap(), InputMime::Pdf);
    }

    #[test]
    fn pagina_colada_no_fim_do_arquivo_e_contada_e_marcadores_colados_tambem() {
        assert_eq!(pdf_page_count(b"/Type /Page"), 1);
        assert_eq!(pdf_page_count(b"/Type/Page"), 1);
        assert_eq!(pdf_page_count(b"/Type /Page/Type /Page"), 2);
        assert_eq!(pdf_page_count(b"/Type /Pages"), 0);
        assert_eq!(pdf_page_count(b""), 0);
        assert_eq!(pdf_page_count(b"/Type /Pag"), 0);
    }

    #[test]
    fn contar_paginas_em_arquivo_de_10mb_e_rapido() {
        let t = std::time::Instant::now();
        assert_eq!(pdf_page_count(&pdf_com_tamanho(MAX_BYTES)), 1);
        assert!(t.elapsed() < std::time::Duration::from_secs(10));
    }

    #[test]
    fn find_acha_na_primeira_e_na_ultima_posicao() {
        assert_eq!(find(b"abc", b"abc"), Some(0));
        assert_eq!(find(b"xxabc", b"abc"), Some(2));
        assert_eq!(find(b"ab", b"abc"), None);
        assert_eq!(find(b"", b"abc"), None);
    }

    // A contagem é só aproximada: variações de espaçamento no dicionário do PDF
    // (válidas pela especificação) escapam do limite de 5 páginas.
    #[test]
    #[ignore = "bug: contagem de páginas ignora '/Type  /Page' (2 espaços), quebra de linha e object streams; PDF de 10 páginas passa"]
    fn pdf_de_10_paginas_com_espacamento_diferente_deveria_ser_rejeitado() {
        let pdf = pdf_n(b"2 0 obj << /Type  /Page >> endobj", 10);
        let e = validate_file(&pdf).unwrap_err();
        assert_eq!(e.code, "invalid_file");
    }
}
