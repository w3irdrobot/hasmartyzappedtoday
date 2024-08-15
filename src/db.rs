use anyhow::Result;
use sqlx::{migrate::Migrator, SqlitePool};
use time::OffsetDateTime;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"

pub async fn connect_database(path: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(path).await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

#[derive(Clone, Debug)]
pub struct Zap {
    pub id: Uuid,
    pub receipt_id: String,
    pub amount: u32,
    pub zapped_at: OffsetDateTime,
}

pub async fn get_most_recent_zap(db: SqlitePool, npub: &str) -> Result<Option<Zap>> {
    let zaps = get_most_recent_zaps(db, npub, 1).await?;

    if zaps.is_empty() {
        Ok(None)
    } else {
        Ok(Some(zaps[0].clone()))
    }
}

pub async fn get_most_recent_zaps(db: SqlitePool, npub: &str, n: u32) -> Result<Vec<Zap>> {
    Ok(sqlx::query_as!(
        Zap,
        r#"SELECT
            id AS "id: Uuid",
            receipt_id,
            amount AS "amount: u32",
            zapped_at AS "zapped_at: OffsetDateTime"
        FROM zaps
        WHERE npub = $1
        ORDER BY zapped_at DESC
        LIMIT $2"#,
        npub,
        n
    )
    .fetch_all(&db)
    .await?)
}

pub async fn zap_already_tracked(db: SqlitePool, npub: &str, receipt_id: &str) -> Result<bool> {
    match sqlx::query!(
        "SELECT zapped_at FROM zaps WHERE npub = $1 AND receipt_id = $2",
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
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO zaps (id, npub, receipt_id, zapped_at, amount)
        VALUES ($1, $2, $3, $4, $5)",
        id,
        npub,
        receipt_id,
        zapped_at,
        amount
    )
    .execute(&db)
    .await?;

    Ok(())
}
