use std::{ops::Add, sync::{Arc, Mutex}, time::{Duration, Instant}};

use ollama_rs::generation::tools::Tool;
use schemars::JsonSchema;
use serde::Deserialize;

/**
 * A tool used to temporarily disable the chatbot from responding to the user.
 * You can bring back the chatbot by saying TimeoutTool::MAGIC_WORD
 **/
pub struct TimeoutTool {
    pub timeout: Arc<Mutex<Instant>>,
}

impl TimeoutTool {
    pub const MAGIC_WORD: &'static str = "cat";
}

#[derive(Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(
        description = "The duration to wait in seconds before being allowed to respond again."// If no specific time is required set it to a large number."
    )]
    timeout: u32,
}

impl Tool for TimeoutTool {
    type Params = Params;

    fn name() -> &'static str {
        "timeout"
    }

    fn description() -> &'static str {
        "Using this tool will make the chatbot not be able to respond for a certain amount of time."
    }

    async fn call(
        &mut self,
        parameters: Self::Params,
    ) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        *self.timeout.lock().unwrap() = Instant::now().add(Duration::from_secs(parameters.timeout.into()));
        println!("TimeoutTool: {}", parameters.timeout);
        Ok("Timeout set to {} seconds".to_string())
    }
}
