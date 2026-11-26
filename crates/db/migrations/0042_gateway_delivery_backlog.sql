CREATE INDEX gateway_webhook_dead_time_idx ON gateway_webhook_deliveries (finished_at)
    WHERE state = 'dead';
