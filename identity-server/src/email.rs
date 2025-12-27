use resend_rs::Resend;
use resend_rs::types::CreateEmailBaseOptions;

pub struct EmailService {
    client: Resend,
    from: String,
    template: String,
}

impl EmailService {
    pub fn new(api_key: &str, from: String, template: String) -> Self {
        Self {
            client: Resend::new(api_key),
            from,
            template,
        }
    }

    pub async fn send_activation_code(&self, to: &str, code: &str) -> Result<(), String> {
        let content = self.template.replace("{{code}}", code);
        let subject = "Ihr CuraDesk Aktivierungscode";

        let email = CreateEmailBaseOptions::new(&self.from, [to], subject).with_html(&content);

        self.client
            .emails
            .send(email)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
