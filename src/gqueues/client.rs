use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use crate::gqueues::models::{Queue, Task};

#[derive(Clone)]
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

        let body = resp.text().await
            .map_err(|e| anyhow!("Failed to read queues response body: {}", e))?;
        log::debug!("getQueues response: {}", body);

        let data: QueuesResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to decode queues response: {}. Body: {}", e, body))?;
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
            return Err(anyhow!("Failed to fetch tasks for queue {}: {}", queue_key, resp.status()));
        }

        let body = resp.text().await
            .map_err(|e| anyhow!("Failed to read tasks response body for queue {}: {}", queue_key, e))?;
        log::debug!("getActiveTasks response for queue {}: {}", queue_key, body);

        let data: TasksResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to decode tasks response for queue {}: {}. Body: {}", queue_key, e, body))?;
        Ok(data.items)
    }

    pub async fn create_task_with_idempotency(&self, text: &str, queue_key: Option<&str>, notes: Option<&str>, idempotency_key: &str) -> Result<Task> {
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

        let body = resp.text().await
            .map_err(|e| anyhow!("Failed to read create task response body: {}", e))?;
        log::debug!("createTask response: {}", body);

        #[derive(Deserialize)]
        struct CreateResponse {
            results: Vec<serde_json::Value>,
        }

        let data: CreateResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to decode create task response: {}. Body: {}", e, body))?;
        let task_json = data.results.first()
            .ok_or_else(|| anyhow!("No task returned in creation response. Body: {}", body))?;
        
        // The API returns { status: "created", task: { ... } }
        let task: Task = serde_json::from_value(task_json["task"].clone())
            .map_err(|e| anyhow!("Failed to parse created task: {}. Item: {}", e, task_json))?;
        
        Ok(task)
    }

    pub async fn create_task(&self, text: &str, queue_key: Option<&str>, notes: Option<&str>) -> Result<Task> {
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        self.create_task_with_idempotency(text, queue_key, notes, &idempotency_key).await
    }
}
