use anyhow::Result;
use chrono::{TimeZone, Utc};
use payments_gateway::config::AppConfig;
use payments_gateway::metrics::aggregator::SlidingMetrics;
use payments_gateway::metrics::event::PaymentEvent;
use payments_gateway::metrics::history_repo::MetricsHistoryRepo;
use payments_gateway::metrics::store_redis::MetricsHotStore;
use redis::streams::StreamReadReply;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = AppConfig::from_env();
    let consumer_name = std::env::var("METRICS_CONSUMER_NAME").unwrap_or_else(|_| "metrics-worker-1".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database_url)
        .await?;

    let history_repo = MetricsHistoryRepo { pool };
    let hot_store = MetricsHotStore::new(&cfg.redis_url)?;
    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let mut conn = redis_client.get_multiplexed_async_connection().await?;

    let _: redis::RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&cfg.stream_key)
        .arg(&cfg.stream_group)
        .arg("0")
        .arg("MKSTREAM")
        .query_async(&mut conn)
        .await;

    let mut agg = SlidingMetrics::default();
    let windows = [1_i64, 5, 15, 60];

    loop {
        let reply: StreamReadReply = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(&cfg.stream_group)
            .arg(&consumer_name)
            .arg("COUNT")
            .arg(200)
            .arg("BLOCK")
            .arg(2000)
            .arg("STREAMS")
            .arg(&cfg.stream_key)
            .arg(">")
            .query_async(&mut conn)
            .await
            .unwrap_or(StreamReadReply { keys: vec![] });

        if reply.keys.is_empty() {
            continue;
        }

        for stream_key in reply.keys {
            for id in stream_key.ids {
                let raw = id
                    .map
                    .get("event")
                    .and_then(|v| redis::from_redis_value::<String>(v).ok());

                let Some(raw_json) = raw else {
                    continue;
                };

                let Ok(event_value) = serde_json::from_str::<serde_json::Value>(&raw_json) else {
                    continue;
                };
                let Ok(event) = serde_json::from_value::<PaymentEvent>(event_value) else {
                    continue;
                };

                agg.ingest(&event);
                let now = Utc::now();

                for key in agg.keys() {
                    for window in windows {
                        if let Some(metric) = agg.compute(&key, window, now) {
                            hot_store.write_metric(&key, window, &metric).await?;
                            let minute = now.timestamp() - (now.second() as i64);
                            let snapshot = Utc.timestamp_opt(minute, 0).single().unwrap_or(now);
                            history_repo
                                .insert_snapshot(snapshot, &key, window as i32, &metric)
                                .await?;
                        }
                    }
                }

                let _: i64 = redis::cmd("XACK")
                    .arg(&cfg.stream_key)
                    .arg(&cfg.stream_group)
                    .arg(id.id)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or(0);
            }
        }
    }
}
