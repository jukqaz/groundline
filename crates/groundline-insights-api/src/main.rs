#![forbid(unsafe_code)]

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match groundline_insights_api::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("groundline_insights_api_failed: {error}");
            ExitCode::FAILURE
        }
    }
}
