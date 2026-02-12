use std::sync::Arc;
use crate::strategies::{ApiStrategy, EmailData};

pub struct SmtpSession {
    from: Option<String>,
    to: Vec<String>,
    data: Option<String>,
    pub expecting_data: bool,
    strategies: Arc<Vec<ApiStrategy>>,
}

impl SmtpSession {
    pub fn new(strategies: Arc<Vec<ApiStrategy>>) -> Self {
        Self {
            from: None,
            to: Vec::new(),
            data: None,
            expecting_data: false,
            strategies,
        }
    }

    fn reset(&mut self) {
        self.from = None;
        self.to.clear();
        self.data = None;
        self.expecting_data = false;
    }

    pub async fn handle_command(&mut self, line: &str) -> String {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return "500 Syntax error\r\n".to_string();
        }

        let command = parts[0].to_uppercase();

        match command.as_str() {
            "EHLO" | "HELO" => {
                self.reset();
                "250 Hello\r\n".to_string()
            }
            "MAIL" => {
                if parts.len() < 2 || !parts[1].to_uppercase().starts_with("FROM:") {
                    return "500 Syntax error\r\n".to_string();
                }
                let from = parts[1][5..].trim().trim_start_matches('<').trim_end_matches('>').to_string();
                self.from = Some(from);
                "250 OK\r\n".to_string()
            }
            "RCPT" => {
                if self.from.is_none() {
                    return "503 Need MAIL command first\r\n".to_string();
                }
                if parts.len() < 2 || !parts[1].to_uppercase().starts_with("TO:") {
                    return "500 Syntax error\r\n".to_string();
                }
                let to = parts[1][3..].trim().trim_start_matches('<').trim_end_matches('>').to_string();
                self.to.push(to);
                "250 OK\r\n".to_string()
            }
            "DATA" => {
                if self.from.is_none() || self.to.is_empty() {
                    return "503 Need MAIL and RCPT commands first\r\n".to_string();
                }
                self.expecting_data = true;
                "354 End data with <CR><LF>.<CR><LF>\r\n".to_string()
            }
            "QUIT" => {
                "221 Bye\r\n".to_string()
            }
            "RSET" => {
                self.reset();
                "250 OK\r\n".to_string()
            }
            "NOOP" => {
                "250 OK\r\n".to_string()
            }
            _ => {
                "500 Command not recognized\r\n".to_string()
            }
        }
    }

    pub async fn handle_data(&mut self, data: String) -> String {
        tracing::info!("Received email data, length: {} bytes", data.len());
        
        // Log first 500 chars of raw data to see email structure
        let preview: String = data.chars().take(500).collect();
        tracing::debug!("Raw email data preview:\n{}", preview);
        
        // Log content type from headers
        for line in data.lines().take(30) {
            if line.to_lowercase().starts_with("content-type:") {
                tracing::info!("Email Content-Type: {}", line);
            }
            if line.to_lowercase().starts_with("mime-version:") {
                tracing::info!("Email MIME-Version: {}", line);
            }
        }
        
        self.data = Some(data);
        self.expecting_data = false;

        if let Some(ref from) = self.from {
            let subject = extract_subject(self.data.as_ref().unwrap_or(&String::new()));
            
            let email_data = EmailData {
                from: from.clone(),
                to: self.to.clone(),
                subject,
                body: self.data.clone().unwrap_or_default(),
                raw_data: self.data.clone().unwrap_or_default(),
            };

            // Send to all configured strategies
            for strategy in self.strategies.iter() {
                match strategy.send_email(email_data.clone()).await {
                    Ok(()) => {
                        tracing::info!("Email successfully forwarded via {} strategy", strategy.name());
                    }
                    Err(err) => {
                        tracing::error!("Failed to forward email via {}: {}", strategy.name(), err);
                    }
                }
            }
        }

        self.reset();
        "250 OK\r\n".to_string()
    }
}

fn extract_subject(data: &str) -> String {
    data.lines()
        .find(|line| line.to_lowercase().starts_with("subject:"))
        .map(|line| line.trim_start_matches("Subject:").trim().to_string())
        .unwrap_or_else(|| "No Subject".to_string())
}
