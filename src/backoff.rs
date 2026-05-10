const MAX_DELAY_BETWEEN_TRIES: std::time::Duration = std::time::Duration::from_secs(5);

pub(crate) fn backoff_default() -> backon::ExponentialBuilder {
    backon::ExponentialBuilder::default()
        .with_max_delay(MAX_DELAY_BETWEEN_TRIES)
        .with_max_times(4)
}
