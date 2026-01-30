use std::{mem, str::FromStr};

use jiff::{
    civil::DateTime, tz::TimeZone, SignedDurationRound, Timestamp, TimestampRound, Unit, Zoned,
};

use crate::{
    decode::Decode,
    encode::{Encode, IsNull},
    error::BoxDynError,
    types::Type,
    PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueFormat, PgValueRef, Postgres,
};

const PG_EPOCH_MICROSECONDS: i64 = 946_684_800 * 1_000_000;
const PG_EPOCH_DATETIME: DateTime = jiff::civil::datetime(2000, 1, 1, 0, 0, 0, 0);

impl Type<Postgres> for DateTime {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMP
    }
}

impl Type<Postgres> for Zoned {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMPTZ
    }
}

impl PgHasArrayType for DateTime {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMP_ARRAY
    }
}

impl PgHasArrayType for Zoned {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMPTZ_ARRAY
    }
}

impl Encode<'_, Postgres> for DateTime {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let microseconds_since_pg_epoch: i64 = self
            .duration_since(PG_EPOCH_DATETIME)
            // My understanding is that Postgres usually truncates, so rounding may not be needed.
            .round(SignedDurationRound::new().smallest(Unit::Microsecond))
            .expect("rounding to succeed")
            .as_micros()
            .try_into()
            .expect("datetime to be within i64 range");

        Encode::<Postgres>::encode(microseconds_since_pg_epoch, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for DateTime {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format {
            PgValueFormat::Binary => {
                let microseconds_since_pg_epoch: i64 = Decode::<Postgres>::decode(value)?;

                let datetime = Timestamp::from_microsecond(
                    PG_EPOCH_MICROSECONDS + microseconds_since_pg_epoch,
                )?
                .to_zoned(TimeZone::UTC)
                .datetime();

                Ok(datetime)
            }

            PgValueFormat::Text => {
                let datetime = DateTime::from_str(value.as_str()?)?;

                Ok(datetime)
            }
        }
    }
}

impl Encode<'_, Postgres> for Zoned {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let microseconds_since_unix_epoch = self
            // The Zoned::timestamp is always stored as UTC
            .timestamp()
            .round(TimestampRound::new().smallest(Unit::Microsecond))?
            .as_microsecond();

        let microseconds_since_pg_epoch = microseconds_since_unix_epoch - PG_EPOCH_MICROSECONDS;

        Encode::<Postgres>::encode(microseconds_since_pg_epoch, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for Zoned {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format {
            PgValueFormat::Binary => {
                let microseconds_since_pg_epoch: i64 = Decode::<Postgres>::decode(value)?;

                let dt = Timestamp::from_microsecond(
                    PG_EPOCH_MICROSECONDS + microseconds_since_pg_epoch,
                )?
                .to_zoned(TimeZone::UTC);

                Ok(dt)
            }

            PgValueFormat::Text => {
                let datetime = DateTime::from_str(value.as_str()?)?;

                Ok(datetime.in_tz("UTC")?)
            }
        }
    }
}
