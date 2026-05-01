use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use crate::models::{Queue, Task};

pub struct GqueuesClient {
    client: Client,
    base_url: String,
    access_token: String,
}

#[derive(Deserialize)]
struct QueuesResponse {
    personal: Option<Vec<Queue>>,
    team: Option<Vec<Queue>>,
    shared: Option<Vec<Queue>>,
}

#[derive(Deserialize)]
struct TasksResponse {
    items: Vec<Task>,
    #[serde(rename = "nextCursor")]
    _next_cursor: Option<String>,
}

impl GqueuesClient {
    pub fn new(base_url: String, access_token: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            access_token,
        }
    }

    pub async fn get_queues(&self) -> Result<Vec<Queue>> {
        let url = format!("{}/v0?action=getQueues", self.base_url);
        let resp = self.client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch queues: {}", resp.status()));
        }

        let data: QueuesResponse = resp.json().await?;
        let mut all_queues = Vec::new();
        if let Some(mut q) = data.personal { all_queues.append(&mut q); }
        if let Some(mut q) = data.team { all_queues.append(&mut q); }
        if let Some(mut q) = data.shared { all_queues.append(&mut q); }
        
        Ok(all_queues)
    }

    pub async fn get_tasks(&self, queue_key: &str) -> Result<Vec<Task>> {
        let url = format!("{}/v0?action=getActiveTasks&queueKey={}", self.base_url, queue_key);
        let resp = self.client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch tasks: {}", resp.status()));
        }

        let data: TasksResponse = resp.json().await?;
        Ok(data.items)
    }

    pub async fn create_task(&self, text: &str, queue_key: Option<&str>, notes: Option<&str>) -> Result<Task> {
        let url = format!("{}/v0", self.base_url);
        let mut instruction = serde_json::json!({
            "text": text,
            "parseQuickAddSyntax": true,
        });
        
        if let Some(qk) = queue_key {
            instruction["queueKey"] = serde_json::json!(qk);
        }
        if let Some(n) = notes {
            instruction["notes"] = serde_json::json!(n);
        }

        let body = serde_json::json!({
            "action": "createTask",
            "instructions": [instruction]
        });

        // Generate a simple idempotency key (random for now, could be deterministic)
        let idempotency_key = uuid::Uuid::new_v4().to_string();

        let resp = self.client
            .post(url)
            .bearer_auth(&self.access_token)
            .header("Idempotency-Key", idempotency_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to create task: {}", resp.status()));
        }

        #[derive(Deserialize)]
        struct CreateResponse {
            items: Vec<serde_json::Value>,
        }

        let data: CreateResponse = resp.json().await?;
        let task_json = data.items.first()
            .ok_or_else(|| anyhow!("No task returned in creation response"))?;
        
        // The API returns { status: "created", task: { ... } }
        let task: Task = serde_json::from_value(task_json["task"].clone())?;
        
        Ok(task)
    }
}
