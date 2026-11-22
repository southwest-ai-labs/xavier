use crate::mesh::tokenomics::wallet::VestingState;
use jiff::Timestamp;

pub struct VestingEngine;

/// Helper function to return the number of days in a given Gregorian month and year.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

impl VestingEngine {
    /// Calculates the total releasable amount for a given vesting state at current time.
    pub fn calculate_releasable(state: &VestingState) -> u64 {
        Self::calculate_releasable_at(state, Timestamp::now())
    }

    /// Calculates the total releasable amount for a given vesting state at a specified timestamp.
    pub fn calculate_releasable_at(state: &VestingState, now: Timestamp) -> u64 {
        let start = Timestamp::from_second(state.start_timestamp).unwrap_or(now);

        let months_passed = Self::calculate_months_passed(start, now);

        let tier = state.tier;
        let release_50 = tier.release_50_month();
        let release_100 = tier.release_100_month();

        let total_eligible = if months_passed >= release_100 {
            state.amount_total
        } else if months_passed >= release_50 {
            state.amount_total / 2
        } else {
            0
        };

        total_eligible.saturating_sub(state.amount_released)
    }

    /// Calculates the full calendar months passed between start and now,
    /// correctly accounting for day-of-month anniversaries and differing month lengths.
    pub fn calculate_months_passed(start: Timestamp, now: Timestamp) -> u32 {
        let start_z = start.to_zoned(jiff::tz::TimeZone::UTC);
        let now_z = now.to_zoned(jiff::tz::TimeZone::UTC);

        let raw_months = (now_z.year() as i32 - start_z.year() as i32) * 12
            + (now_z.month() as i32 - start_z.month() as i32);

        let target_day =
            (start_z.day() as u32).min(days_in_month(now_z.year() as i32, now_z.month() as u32));
        let adjusted_months = if (now_z.day() as u32) < target_day {
            raw_months - 1
        } else {
            raw_months
        };

        adjusted_months.max(0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::tokenomics::wallet::InvestmentTier;

    fn ts(y: i16, m: i8, d: i8, h: i8, min: i8, s: i8) -> Timestamp {
        jiff::civil::date(y, m, d)
            .at(h, min, s, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn test_exact_same_day_one_month_later() {
        let start = ts(2026, 1, 15, 12, 0, 0);
        let now = ts(2026, 2, 15, 12, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, now), 1);

        // Multiple months on same day
        let now_3m = ts(2026, 4, 15, 12, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, now_3m), 3);

        // One year later same day
        let now_1y = ts(2027, 1, 15, 12, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, now_1y), 12);
    }

    #[test]
    fn test_one_day_before_anniversary() {
        let start = ts(2026, 1, 15, 12, 0, 0);
        let now = ts(2026, 2, 14, 23, 59, 59);
        assert_eq!(VestingEngine::calculate_months_passed(start, now), 0);

        // 1 day before 2nd month anniversary
        let now_2m_minus_1d = ts(2026, 3, 14, 23, 59, 59);
        assert_eq!(
            VestingEngine::calculate_months_passed(start, now_2m_minus_1d),
            1
        );
    }

    #[test]
    fn test_jan_31_to_feb_non_leap_year() {
        // 2026 is not a leap year, February has 28 days
        let start = ts(2026, 1, 31, 10, 0, 0);

        // Feb 1 (only 1 day passed): 0 months
        let feb_1 = ts(2026, 2, 1, 10, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, feb_1), 0);

        // Feb 27 (1 day before end of month): 0 months
        let feb_27 = ts(2026, 2, 27, 23, 59, 59);
        assert_eq!(VestingEngine::calculate_months_passed(start, feb_27), 0);

        // Feb 28 (last day of month): 1 month passed
        let feb_28 = ts(2026, 2, 28, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, feb_28), 1);

        // March 30: 1 month passed (March has 31 days)
        let mar_30 = ts(2026, 3, 30, 23, 59, 59);
        assert_eq!(VestingEngine::calculate_months_passed(start, mar_30), 1);

        // March 31: 2 months passed
        let mar_31 = ts(2026, 3, 31, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, mar_31), 2);
    }

    #[test]
    fn test_jan_31_to_feb_leap_year() {
        // 2024 is a leap year, February has 29 days
        let start = ts(2024, 1, 31, 12, 0, 0);

        // Feb 28 (not last day in leap year): 0 months
        let feb_28 = ts(2024, 2, 28, 23, 59, 59);
        assert_eq!(VestingEngine::calculate_months_passed(start, feb_28), 0);

        // Feb 29 (last day of leap February): 1 month passed
        let feb_29 = ts(2024, 2, 29, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_months_passed(start, feb_29), 1);
    }

    #[test]
    fn test_full_cliff_unlock_50_and_100_percent() {
        // Bronze tier: release_50_month = 2, release_100_month = 4
        let start = ts(2026, 1, 15, 0, 0, 0);
        let total_amount = 1_000_000u64;

        let mut state = VestingState {
            tier: InvestmentTier::Bronze,
            amount_total: total_amount,
            amount_released: 0,
            start_timestamp: start.as_second(),
            last_claim_timestamp: start.as_second(),
        };

        // Month 0 (start day): 0 releasable
        assert_eq!(VestingEngine::calculate_releasable_at(&state, start), 0);

        // Month 1 (Feb 15): 0 releasable (before cliff)
        let m1 = ts(2026, 2, 15, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_releasable_at(&state, m1), 0);

        // Month 2 minus 1 day (Mar 14): 0 releasable
        let m2_minus_1d = ts(2026, 3, 14, 23, 59, 59);
        assert_eq!(
            VestingEngine::calculate_releasable_at(&state, m2_minus_1d),
            0
        );

        // Month 2 (Mar 15): 50% cliff reached -> 500,000 releasable
        let m2 = ts(2026, 3, 15, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_releasable_at(&state, m2), 500_000);

        // If 200,000 is released, releasable becomes 300,000
        state.amount_released = 200_000;
        assert_eq!(VestingEngine::calculate_releasable_at(&state, m2), 300_000);

        // If full 50% (500,000) was already released, releasable at m2/m3 is 0
        state.amount_released = 500_000;
        let m3 = ts(2026, 4, 15, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_releasable_at(&state, m3), 0);

        // Month 4 (May 15): 100% cliff reached -> eligible 1,000,000 - 500,000 released = 500,000
        let m4 = ts(2026, 5, 15, 0, 0, 0);
        assert_eq!(VestingEngine::calculate_releasable_at(&state, m4), 500_000);

        // If 0 was previously released, month 4 yields full 100% (1,000,000)
        state.amount_released = 0;
        assert_eq!(
            VestingEngine::calculate_releasable_at(&state, m4),
            1_000_000
        );
    }
}
