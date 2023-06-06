use sqlx::types::time::{OffsetDateTime, PrimitiveDateTime};

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

    pub fn elapsed(&self) {
        let elapsed = now() - self.begin;
        info!(
            "Elapsed {} s ",
            elapsed.whole_milliseconds() as f64 / 1000.0
        )
    }
}
