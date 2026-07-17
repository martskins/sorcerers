use anyhow::{Context, Result};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};

pub struct EmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl EmailSender {
    pub fn from_env() -> Result<Self> {
        let relay = std::env::var("SMTP_RELAY").context("SMTP_RELAY must be set")?;
        let from = std::env::var("SMTP_FROM")
            .context("SMTP_FROM must be set")?
            .parse()
            .context("SMTP_FROM must be a valid mailbox")?;
        let username = std::env::var("SMTP_USERNAME").context("SMTP_USERNAME must be set")?;
        let password = std::env::var("SMTP_PASSWORD").context("SMTP_PASSWORD must be set")?;
        let port = std::env::var("SMTP_PORT")
            .ok()
            .map(|value| value.parse().context("SMTP_PORT must be a number"))
            .transpose()?
            .unwrap_or(587);
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&relay)
            .context("SMTP_RELAY must be a valid hostname")?
            .port(port)
            .credentials(Credentials::new(username, password))
            .build();
        Ok(Self { transport, from })
    }

    pub async fn send_confirmation_code(&self, email: &str, code: &str) -> Result<()> {
        let message = Message::builder()
            .from(self.from.clone())
            .to(email.parse().context("confirmation email is invalid")?)
            .subject("Confirm your Sorcerers email")
            .body(format!(
                "Your Sorcerers confirmation code is {code}. It expires in 15 minutes.\n\nIf you did not create an account, you can ignore this email."
            ))
            .context("failed to build confirmation email")?;
        self.transport.send(message).await?;
        // .context("failed to deliver confirmation email")?;
        Ok(())
    }
}
