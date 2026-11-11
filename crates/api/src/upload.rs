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
