use sqlx::types::time::{OffsetDateTime, PrimitiveDateTime};
use std::fmt::Display;

pub fn now() -> PrimitiveDateTime {
    let now = OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}

pub struct Ticker {
    begin: PrimitiveDateTime,
}

impl Ticker {
    pub fn new() -> Ticker {
        Ticker { begin: now() }
    }

    pub fn elapsed(&self, msg: impl Display) {
        let elapsed = now() - self.begin;
        info!(
            "Took {} seconds {}",
            elapsed.whole_milliseconds() as f64 / 1000.0,
            msg
        )
    }
}
