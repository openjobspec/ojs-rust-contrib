use ojs::{JobContext, Worker};
use serde_json::json;

#[tokio::main]
async fn main() -> ojs::Result<()> {
    tracing_subscriber::fmt::init();

    let worker = Worker::builder()
        .url("http://localhost:8080")
        .queues(vec!["default"])
        .concurrency(5)
        .build()?;

    worker
        .register("email.send", |ctx: JobContext| async move {
            let to: String = ctx.job.arg("to")?;
            println!("Sending email to {to}");
            Ok(json!({"status": "sent", "to": to}))
        })
        .await;

    println!("Worker started, processing jobs...");
    worker.start().await?;

    Ok(())
}
