use chrono::Local;
use ollama_rs::generation::tools::Tool;
use schemars::JsonSchema;
use serde::Deserialize;

/**
 * The AI can use this tool to get the current local time
 */
pub struct TimeTool {}

#[derive(Deserialize, JsonSchema)]
pub struct Params {}

impl Tool for TimeTool {
    type Params = Params;

    fn name() -> &'static str {
        "timetool"
    }

    fn description() -> &'static str {
        "Returns the current time."
    }

    async fn call(
        &mut self,
        parameters: Self::Params,
    ) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let date = Local::now().format("%Y-%m-%d][%H:%M:%S");
        println!("TimeTool: {}", date);
        Ok(format!("{}", date))
    }
}
