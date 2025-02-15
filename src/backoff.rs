const MAX_INTERVAL_BETWEEN_TRIES: std::time::Duration = std::time::Duration::from_secs(5);

#[allow(unused)]
pub(crate) fn backoff_infinite() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoffBuilder::new()
        .with_max_interval(MAX_INTERVAL_BETWEEN_TRIES)
        .with_max_elapsed_time(None)
        .build()
}

pub(crate) fn backoff_default() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoffBuilder::new()
        .with_max_interval(MAX_INTERVAL_BETWEEN_TRIES)
        .with_max_elapsed_time(Some(std::time::Duration::from_secs(30)))
        .build()
}
