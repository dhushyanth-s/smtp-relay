use super::EmailData;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// Generic webhook strategy for sending emails to any HTTP endpoint
#[derive(Debug, Clone)]
pub struct WebhookStrategy {
    client: reqwest::Client,
    url: String,
    headers: HeaderMap,
}

#[derive(serde::Serialize)]
struct WebhookPayload {
    from: String,
    to: Vec<String>,
    subject: String,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
}

impl WebhookStrategy {
    pub fn new(url: String, extra_headers: Option<Vec<(String, String)>>) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        
        // Add any extra headers
        if let Some(extra) = extra_headers {
            for (key, value) in extra {
                if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(header_value) = HeaderValue::from_str(&value) {
                        headers.insert(header_name, header_value);
                    }
                }
            }
        }
        
        Ok(Self {
            client,
            url,
            headers,
        })
    }
    
    pub async fn send_email(&self, email: EmailData) -> anyhow::Result<()> {
        let payload = WebhookPayload {
            from: email.from,
            to: email.to,
            subject: email.subject,
            body: email.body.clone(),
            html: extract_html(&email.raw_data),
        };
        
        let response = self.client
            .post(&self.url)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Webhook request failed: {} - {}", status, text);
        }
        
        tracing::info!("Webhook request successful: {}", response.status());
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        "webhook"
    }
}

/// Attempt to extract HTML content from email body
fn extract_html(raw_data: &str) -> Option<String> {
    // Look for Content-Type: text/html and extract the body
    let lines: Vec<&str> = raw_data.lines().collect();
    let mut in_html = false;
    let mut html_content = Vec::new();
    
    for line in &lines {
        if line.to_lowercase().contains("content-type: text/html") {
            in_html = true;
            continue;
        }
        
        if in_html {
            if line.is_empty() {
                continue;
            }
            // Check for boundary or end of section
            if line.starts_with("--") || line.starts_with("Content-") {
                break;
            }
            html_content.push(line.to_string());
        }
    }
    
    if html_content.is_empty() {
        None
    } else {
        Some(html_content.join("\n"))
    }
}
