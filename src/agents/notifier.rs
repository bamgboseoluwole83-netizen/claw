use reqwest::Client;
use tracing::info;

#[derive(Clone)]
pub struct NtfyNotifier {
    topic: String,
    client: Client,
}

impl NtfyNotifier {
    /// topic: the unique topic name you'll subscribe to (e.g., "my_agent_alerts")
    pub fn new(topic: String) -> Self {
        Self {
            topic,
            client: Client::new(),
        }
    }

    /// Send a plain text notification.
    pub async fn send(&self, title: &str, message: &str) {
        let url = format!("https://ntfy.sh/{}", self.topic);
        let res = self.client
            .post(&url)
            .header("Title", title)
            .body(message.to_string())
            .send()
            .await;
        match res {
            Ok(_) => info!("📤 ntfy alert sent."),
            Err(e) => info!("❌ ntfy failed: {e}"),
        }
    }

    #[allow(dead_code)]
    pub async fn send_poc(&self, profit: &str, report_path: &str) {
        let body = std::fs::read_to_string(report_path)
            .unwrap_or_else(|_| "PoC content unavailable.".to_string());
        let title = format!("💥 Critical – profit: {}", profit);
        self.send(&title, &body).await;
    }
}
