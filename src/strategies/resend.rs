use super::EmailData;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::header::{HeaderMap, HeaderValue};

/// Resend API strategy for sending emails via Resend
/// https://resend.com/docs/api-reference/emails/send-email
#[derive(Debug, Clone)]
pub struct ResendStrategy {
    client: reqwest::Client,
    #[allow(dead_code)]
    api_key: String,
}

#[derive(serde::Serialize)]
struct ResendPayload {
    from: String,
    to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<Attachment>>,
}

#[derive(serde::Serialize)]
struct Attachment {
    filename: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
}

impl ResendStrategy {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            api_key
        })
    }

    pub async fn send_email(&self, email: EmailData) -> anyhow::Result<()> {
        tracing::info!("Resend strategy processing email from: {}", email.from);

        // Parse email content
        let (text, html, attachments) = parse_email(&email.raw_data);

        tracing::info!(
            "Parsed email - Text: {}, HTML: {}, Attachments: {}",
            if text.is_some() { "yes" } else { "no" },
            if html.is_some() { "yes" } else { "no" },
            attachments.as_ref().map(|a| a.len()).unwrap_or(0)
        );

        let payload = ResendPayload {
            from: email.from.clone(),
            to: email.to,
            subject: Some(email.subject),
            text,
            html,
            reply_to: Some(email.from),
            attachments,
        };

        let response = self
            .client
            .post("https://api.resend.com/emails")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Resend API request failed: {} - {}", status, text);
        }

        let resend_response: serde_json::Value = response.json().await?;
        tracing::info!(
            "Resend email sent successfully. ID: {}",
            resend_response
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );

        Ok(())
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        "resend"
    }
}

/// Parse email and extract content
/// Returns (text, html, attachments)
fn parse_email(raw_data: &str) -> (Option<String>, Option<String>, Option<Vec<Attachment>>) {
    // Split headers from body
    let (headers, body) = match split_headers_body(raw_data) {
        Some((h, b)) => (h, b),
        None => return (Some(raw_data.to_string()), None, None),
    };

    // Get content type
    let content_type = get_header(&headers, "content-type").to_lowercase();
    let is_multipart = content_type.starts_with("multipart/");

    if !is_multipart {
        // Simple email - use body as-is
        let is_html = content_type.contains("text/html");
        let decoded_body = decode_body(body, &headers);
        
        if is_html {
            return (None, Some(decoded_body), None);
        } else {
            return (Some(decoded_body), None, None);
        }
    }

    // Multipart email - parse parts
    parse_multipart(body, &headers)
}

/// Split email into headers and body
fn split_headers_body(raw_data: &str) -> Option<(&str, &str)> {
    if let Some(pos) = raw_data.find("\r\n\r\n") {
        return Some((&raw_data[..pos], &raw_data[pos + 4..]));
    }
    if let Some(pos) = raw_data.find("\n\n") {
        return Some((&raw_data[..pos], &raw_data[pos + 2..]));
    }
    None
}

/// Get header value (case-insensitive)
fn get_header(headers: &str, name: &str) -> String {
    let name_lower = name.to_lowercase();
    headers
        .lines()
        .find(|line| line.to_lowercase().starts_with(&format!("{}:", name_lower)))
        .map(|line| line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
        .unwrap_or_default()
}

/// Decode body based on transfer encoding
fn decode_body(body: &str, headers: &str) -> String {
    let encoding = get_header(headers, "content-transfer-encoding").to_lowercase();
    
    match encoding.as_str() {
        "quoted-printable" => decode_quoted_printable(body),
        "base64" => {
            // Try to decode base64, fallback to raw if it fails
            match BASE64.decode(body.replace(['\r', '\n'], "")) {
                Ok(decoded) => String::from_utf8_lossy(&decoded).to_string(),
                Err(_) => body.to_string(),
            }
        }
        _ => body.to_string(),
    }
}

/// Parse multipart email into parts
fn parse_multipart(body: &str, headers: &str) -> (Option<String>, Option<String>, Option<Vec<Attachment>>) {
    // Get boundary
    let boundary = get_header(headers, "content-type")
        .split("boundary=")
        .nth(1)
        .map(|b| b.trim().trim_matches('"').trim_matches('\''))
        .map(|b| format!("--{}", b));

    let Some(boundary) = boundary else {
        tracing::warn!("Multipart email missing boundary");
        return (Some(body.to_string()), None, None);
    };

    let mut text_parts = Vec::new();
    let mut html_parts = Vec::new();
    let mut attachments = Vec::new();

    // Split by boundary
    for part in body.split(&boundary) {
        let part = part.trim();
        if part.is_empty() || part == "--" {
            continue;
        }

        // Split part into headers and body
        let Some((part_headers, part_body)) = split_headers_body(part) else {
            continue;
        };

        let part_ct = get_header(part_headers, "content-type").to_lowercase();
        let decoded = decode_body(part_body, part_headers);

        // Check if this is an attachment
        let is_attachment = part_headers.to_lowercase().contains("content-disposition: attachment")
            || part_ct.contains("name=");

        if is_attachment || (!part_ct.contains("text/plain") && !part_ct.contains("text/html")) {
            // It's an attachment or binary content
            if let Some(filename) = extract_filename(part_headers, &part_ct) {
                let content = BASE64.encode(&decoded);
                attachments.push(Attachment {
                    filename,
                    content,
                    content_type: Some(get_header(part_headers, "content-type")),
                });
            }
        } else if part_ct.contains("text/html") {
            html_parts.push(decoded);
        } else if part_ct.contains("text/plain") {
            text_parts.push(decoded);
        }
    }

    let text = if text_parts.is_empty() {
        None
    } else {
        Some(text_parts.join("\n\n"))
    };

    let html = if html_parts.is_empty() {
        None
    } else {
        Some(html_parts.join("<br><br>"))
    };

    let attachments = if attachments.is_empty() {
        None
    } else {
        Some(attachments)
    };

    (text, html, attachments)
}

/// Extract filename from headers
fn extract_filename(headers: &str, content_type: &str) -> Option<String> {
    // Try Content-Disposition first
    let cd = get_header(headers, "content-disposition");
    if let Some(filename) = cd.split("filename=").nth(1) {
        return Some(filename.trim().trim_matches('"').trim_matches('\'').to_string());
    }

    // Try Content-Type name parameter
    if let Some(name) = content_type.split("name=").nth(1) {
        return Some(name.trim().trim_matches('"').trim_matches('\'').to_string());
    }

    None
}

/// Decode quoted-printable
fn decode_quoted_printable(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '=' {
            // Check for soft line break
            if chars.peek() == Some(&'\r') {
                chars.next();
                if chars.peek() == Some(&'\n') {
                    chars.next();
                    continue;
                }
            } else if chars.peek() == Some(&'\n') {
                chars.next();
                continue;
            }

            // Decode hex
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('=');
                    result.push_str(&hex);
                }
            } else {
                result.push('=');
                result.push_str(&hex);
            }
        } else {
            result.push(ch);
        }
    }

    result
}
