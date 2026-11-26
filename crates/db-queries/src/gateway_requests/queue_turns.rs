use sqlx::{PgPool, Postgres, Transaction};

/// A control change commits between submission claims. Polling has its own lane.
pub(crate) async fn lock_submission_lanes(tx: &mut Transaction<'_, Postgres>) -> sqlx::Result<()> {
    for lane in [QueueLane::MockSubmit, QueueLane::ProviderSubmit] {
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtext('gateway_queue_claim'), hashtext($1))",
            lane.key(),
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum QueueLane {
    ProviderSubmit,
    ProviderPoll,
    MockSubmit,
}

impl QueueLane {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::ProviderSubmit => "provider_submit",
            Self::ProviderPoll => "provider_poll",
            Self::MockSubmit => "mock_submit",
        }
    }

    pub(super) async fn begin(
        self,
        pool: &PgPool,
    ) -> sqlx::Result<Option<Transaction<'static, Postgres>>> {
        let mut tx = pool
            .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED")
            .await?;
        // The claim's fresh snapshot must start after obtaining the lane lock.
        let acquired = sqlx::query_scalar!(
            r#"SELECT pg_try_advisory_xact_lock(hashtext('gateway_queue_claim'), hashtext($1)) AS "acquired!""#,
            self.key(),
        )
        .fetch_one(&mut *tx)
        .await?;
        if acquired {
            Ok(Some(tx))
        } else {
            tx.rollback().await?;
            Ok(None)
        }
    }
}
