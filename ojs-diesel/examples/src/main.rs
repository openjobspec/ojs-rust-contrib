use diesel::prelude::*;
use ojs::Client;
use ojs_diesel::{enqueue_to_outbox, OutboxPublisher};
use serde_json::json;

fn create_order(conn: &mut PgConnection, order_id: i64) -> QueryResult<()> {
    conn.transaction(|conn| {
        // In a real app, insert your order here:
        // diesel::insert_into(orders::table).values(...).execute(conn)?;

        // Enqueue a job in the same transaction
        enqueue_to_outbox(conn, "order.process", json!({"order_id": order_id}))?;
        println!("Order {order_id} created with outbox entry");
        Ok(())
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/myapp".to_string());

    let ojs_client = Client::builder()
        .url("http://localhost:8080")
        .build()
        .expect("failed to build OJS client");

    // Start the outbox publisher in the background
    let _publisher = OutboxPublisher::new(ojs_client, &database_url).start();

    // Simulate creating orders with transactional enqueue
    let mut conn = PgConnection::establish(&database_url).expect("failed to connect to database");

    for i in 1..=5 {
        if let Err(e) = create_order(&mut conn, i) {
            eprintln!("Failed to create order {i}: {e}");
        }
    }

    println!("All orders created. Publisher will forward jobs to OJS...");

    // Let the publisher run for a bit
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    println!("Done.");
}
