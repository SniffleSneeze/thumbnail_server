use anyhow::Ok;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read the .env file and build environment variables
    dotenv::dotenv()?;

    // get envi database url string
    let db_url = std::env::var("DATABASE_URL")?;

    // create pool connection
    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(())
}
