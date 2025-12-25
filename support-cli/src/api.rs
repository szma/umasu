use anyhow::{Context, Result};
use reqwest::blocking::{multipart, Client};
use support_common::{CreateCommentRequest, Ticket, TicketDetail, TicketState, UpdateStateRequest};

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    pub fn list_tickets(&self) -> Result<Vec<Ticket>> {
        let resp = self
            .client
            .get(format!("{}/admin/tickets", self.base_url))
            .header("x-api-key", &self.api_key)
            .send()
            .context("Konnte Server nicht erreichen")?;

        if !resp.status().is_success() {
            anyhow::bail!("Server Fehler: {}", resp.status());
        }

        resp.json().context("Ungültige Antwort vom Server")
    }

    pub fn get_ticket(&self, id: i64) -> Result<TicketDetail> {
        let resp = self
            .client
            .get(format!("{}/admin/tickets/{}", self.base_url, id))
            .header("x-api-key", &self.api_key)
            .send()
            .context("Konnte Server nicht erreichen")?;

        if !resp.status().is_success() {
            anyhow::bail!("Server Fehler: {}", resp.status());
        }

        resp.json().context("Ungültige Antwort vom Server")
    }

    pub fn update_state(&self, id: i64, state: TicketState) -> Result<()> {
        let resp = self
            .client
            .put(format!("{}/admin/tickets/{}/state", self.base_url, id))
            .header("x-api-key", &self.api_key)
            .json(&UpdateStateRequest { state })
            .send()
            .context("Konnte Server nicht erreichen")?;

        if !resp.status().is_success() {
            anyhow::bail!("Server Fehler: {}", resp.status());
        }

        Ok(())
    }

    pub fn add_comment(&self, ticket_id: i64, text: String) -> Result<()> {
        let resp = self
            .client
            .post(format!(
                "{}/admin/tickets/{}/comments",
                self.base_url, ticket_id
            ))
            .header("x-api-key", &self.api_key)
            .json(&CreateCommentRequest { text })
            .send()
            .context("Konnte Server nicht erreichen")?;

        if !resp.status().is_success() {
            anyhow::bail!("Server Fehler: {}", resp.status());
        }

        Ok(())
    }

    pub fn download_zip(&self, id: i64) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get(format!("{}/admin/tickets/{}/zip", self.base_url, id))
            .header("x-api-key", &self.api_key)
            .send()
            .context("Konnte Server nicht erreichen")?;

        if !resp.status().is_success() {
            anyhow::bail!("Server Fehler: {}", resp.status());
        }

        resp.bytes()
            .map(|b| b.to_vec())
            .context("Fehler beim Herunterladen")
    }

    pub fn create_ticket(&self, description: String, zip_data: Vec<u8>) -> Result<Ticket> {
        let form = multipart::Form::new()
            .text("description", description)
            .part(
                "zip",
                multipart::Part::bytes(zip_data)
                    .file_name("report.zip")
                    .mime_str("application/zip")?,
            );

        let resp = self
            .client
            .post(format!("{}/tickets", self.base_url))
            .header("x-api-key", &self.api_key)
            .multipart(form)
            .send()
            .context("Konnte Server nicht erreichen")?;

        if !resp.status().is_success() {
            anyhow::bail!("Server Fehler: {}", resp.status());
        }

        resp.json().context("Ungültige Antwort vom Server")
    }
}
