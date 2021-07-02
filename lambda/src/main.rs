use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {
    oauth_token: String,
    oauth_verifier: String,
}

#[derive(Serialize)]
struct Response {
    message_id: Option<String>,
    sequence_number: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    let func = lambda_runtime::handler_fn(handler);
    lambda_runtime::run(func).await
}

async fn handler(event: Request, _ctx: lambda_runtime::Context) -> anyhow::Result<Response> {
    let client = aws_sqs::Client::from_env();
    // Get SQS queue based on name set by client
    let queue = client
        .get_queue_url()
        .queue_name(format!("bullhorn-{}", event.oauth_token))
        .send()
        .await?;

    // Send oauth_verifier to client so it can retrieve user token/secret
    let res = client
        .send_message()
        .message_body(event.oauth_verifier)
        .set_queue_url(queue.queue_url)
        .send()
        .await?;
    let response = Response {
        message_id: res.message_id,
        sequence_number: res.sequence_number,
    };
    Ok(response)
}
