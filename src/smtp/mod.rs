pub mod session;

use std::sync::Arc;
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter}, net::TcpStream};

use crate::strategies::ApiStrategy;
use session::SmtpSession;

pub async fn handle_connection(mut stream: TcpStream, strategies: Arc<Vec<ApiStrategy>>) -> anyhow::Result<()> {
    let addr = stream.peer_addr()?;
    tracing::info!("New connection from {}", addr);

    let (reader, writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    writer.write_all(b"220 SMTP Server Ready\r\n").await?;
    writer.flush().await?;

    let mut session = SmtpSession::new(strategies);
    let mut line = String::new();

    loop {
        line.clear();
        
        if session.expecting_data {
            let mut data_lines: Vec<String> = Vec::new();
            
            loop {
                line.clear();
                let bytes_read = reader.read_line(&mut line).await?;
                if bytes_read == 0 {
                    return Ok(());
                }
                
                if line.trim() == "." {
                    break;
                }
                
                if line.starts_with("..") {
                    data_lines.push(line[1..].to_string());
                } else {
                    data_lines.push(line.clone());
                }
            }
            
            let data = data_lines.join("");
            let response = session.handle_data(data).await;
            writer.write_all(response.as_bytes()).await?;
            writer.flush().await?;
            
            if response.starts_with("221") {
                return Ok(());
            }
        } else {
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                return Ok(());
            }

            let trimmed = line.trim();
            tracing::debug!("Received: {}", trimmed);

            let response = session.handle_command(trimmed).await;
            writer.write_all(response.as_bytes()).await?;
            writer.flush().await?;

            if response.starts_with("221") {
                return Ok(());
            }
        }
    }
}
