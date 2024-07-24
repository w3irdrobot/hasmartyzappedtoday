use anyhow::Result;
use sqlx::{migrate::Migrator, SqlitePool};
use time::OffsetDateTime;

static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"

pub async fn connect_database(path: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(path).await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

pub struct Zap {
    pub zapped_at: OffsetDateTime,
}

pub async fn get_most_recent_zap(db: SqlitePool, npub: &str) -> Result<Zap> {
    Ok(sqlx::query_as!(
        Zap,
        r#"SELECT zapped_at AS "zapped_at: OffsetDateTime"
        FROM zaps
        WHERE npub = $1
        ORDER BY zapped_at DESC
        LIMIT 1"#,
        npub
    )
    .fetch_one(&db)
    .await?)
}

pub async fn zap_already_tracked(db: SqlitePool, npub: &str, receipt_id: &str) -> Result<bool> {
    match sqlx::query!(
        "SELECT id FROM zaps WHERE npub = $1 AND receipt_id = $2",
        npub,
        receipt_id
    )
    .fetch_one(&db)
    .await
    {
        Ok(_) => Ok(true),
        Err(sqlx::Error::RowNotFound) => Ok(false),
        Err(err) => Err(err.into()),
    }
}

pub async fn add_zap(
    db: SqlitePool,
    npub: &str,
    receipt_id: &str,
    zapped_at: OffsetDateTime,
    amount: u32,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO zaps (npub, receipt_id, zapped_at, amount)
        VALUES ($1, $2, $3, $4)",
        npub,
        receipt_id,
        zapped_at,
        amount
    )
    .execute(&db)
    .await?;

    Ok(())
}
