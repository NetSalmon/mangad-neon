use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpireTime {
    Short,    // 20 mins
    Medium,   // 7 dats
    Long,     // 30 days
    Extended, // 90 days
    Permanent,
}

impl Default for ExpireTime {
    fn default() -> Self {
        ExpireTime::Long
    }
}

impl ExpireTime {
    pub fn to_duration(&self) -> Option<chrono::Duration> {
        match self {
            ExpireTime::Short => Some(chrono::Duration::minutes(20)),
            ExpireTime::Medium => Some(chrono::Duration::days(7)),
            ExpireTime::Long => Some(chrono::Duration::days(30)),
            ExpireTime::Extended => Some(chrono::Duration::days(90)),
            ExpireTime::Permanent => None,
        }
    }

    pub fn get_expire_time(
        &self,
        create_time: DateTimeWithTimeZone,
    ) -> Option<DateTimeWithTimeZone> {
        let Some(duration) = self.to_duration() else {
            return None;
        };

        Some(create_time + duration)
    }
}
