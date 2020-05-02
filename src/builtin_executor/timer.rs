use std::ops::{Add, Sub};

use chrono::{Date, Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike, TimeZone};

use nature_common::{ConverterParameter, ConverterReturned, DEFAULT_PARA_SEPARATOR, Instance, is_default, NatureError, Result};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Setting {
    /// s(econd), m(inute), h(our), d(ay)
    #[serde(skip_serializing_if = "is_s")]
    #[serde(default = "string_s")]
    unit: String,
    /// When unit is s,m,h,d the value great than 1, it mean interval
    /// When unit is w,M,y the value mean offset. It could be negative, means offset from the end.
    /// - 0 is the first day of the unit.
    /// - week : value must in [-7, 6]
    /// - month : value must in [-20, 19]
    /// - year : value must in [-200, 199]
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    value: i16,
}

static SECOND: i64 = 1000;
static MINUTE: i64 = 1000 * 60;
static HOUR: i64 = 1000 * 60 * 60;
static DAY: i64 = 1000 * 60 * 60 * 24;

impl Setting {
    fn get_time(&self, ins_time: i64) -> Result<(i64, i64)> {
        let time = Local.timestamp_millis(ins_time).naive_local();
        let unit = self.unit.as_ref();
        let rtn: (i64, i64) = match unit {
            "s" => {
                let mut rtn = ins_time - time.timestamp_subsec_millis() as i64;
                if self.value > 1 {
                    let redundant = time.second() % self.value as u32;
                    rtn = rtn - redundant as i64 * SECOND
                };
                (rtn, rtn + SECOND)
            }
            "m" => {
                let mut rtn = ins_time - time.second() as i64 * SECOND - time.timestamp_subsec_millis() as i64;
                if self.value > 1 {
                    let redundant = time.minute() % self.value as u32;
                    rtn = rtn - redundant as i64 * MINUTE
                };
                (rtn, rtn + MINUTE)
            }
            "h" => {
                let mut rtn = ins_time - time.minute() as i64 * MINUTE - time.second() as i64 * SECOND - time.timestamp_subsec_millis() as i64;
                if self.value > 1 {
                    let redundant = time.hour() % self.value as u32;
                    rtn = rtn - redundant as i64 * HOUR
                };
                (rtn, rtn + HOUR)
            }
            "d" => {
                let mut rtn = ins_time - time.hour() as i64 * HOUR - time.minute() as i64 * MINUTE - time.second() as i64 * SECOND - time.timestamp_subsec_millis() as i64;
                if self.value > 1 {
                    let redundant = time.num_days_from_ce() % self.value as i32;
                    rtn = rtn - redundant as i64 * DAY
                };
                (rtn, rtn + DAY)
            }
            "w" => return self.get_week(&time),
            "M" => return self.get_month(&time),
            "y" => return self.get_year(&time),
            _ => {
                let err = format!("timer setting error: unknown unit '{}'", self.unit);
                return Err(NatureError::LogicalError(err));
            }
        };
        Ok(rtn)
    }

    fn get_week(&self, nd: &NaiveDateTime) -> Result<(i64, i64)> {
        if self.value > 6 || self.value < -7 {
            return Err(NatureError::LogicalError("value must in [-7,6]".to_string()));
        }
        let offset = nd.weekday().num_days_from_monday() as i16;
        let mut value = self.value;
        if value < 0 {
            value += 7
        }
        let diff_day = if value <= offset {
            offset - value
        } else {
            7 - value + offset
        };
        let rtn = Local.ymd(nd.year(), nd.month(), nd.day()).sub(Duration::days(diff_day as i64)).and_hms(0, 0, 0).timestamp_millis();
        Ok((rtn, rtn + 7 * DAY))
    }

    fn get_month(&self, nd: &NaiveDateTime) -> Result<(i64, i64)> {
        // check value
        if self.value > 19 || self.value < -20 {
            return Err(NatureError::LogicalError("the `value` must in [-20,19]".to_string()));
        }
        let offset = nd.day0() as i16;
        let this_month = Local.ymd(nd.year(), nd.month(), 1);
        let next_month = get_next_month(&this_month.naive_local());
        let mut value = self.value;
        if value < 0 {
            let days = next_month.sub(this_month).num_days();
            value += days as i16;
        }
        let rtn = if value <= offset {
            // `begin` in this month and `end` in next month
            let begin = Local.ymd(nd.year(), nd.month(), (value + 1) as u32).and_hms(0, 0, 0);
            let left = begin.timestamp_millis();
            let right = if self.value >= 0 {
                next_month.add(Duration::days(self.value as i64)).and_hms(0, 0, 0).timestamp_millis()
            } else {
                let next_next = get_next_month(&next_month.naive_local());
                let end = next_next.sub(Duration::days(-self.value as i64)).and_hms(0, 0, 0);
                end.timestamp_millis()
            };
            (left, right)
        } else {
            // `begin` in previous month and `end` in this month
            if self.value >= 0 {
                let left = get_previous_month(&this_month.naive_local()).add(Duration::days(self.value as i64)).and_hms(0, 0, 0).timestamp_millis();
                let right = this_month.add(Duration::days(self.value as i64)).and_hms(0, 0, 0).timestamp_millis();
                (left, right)
            } else {
                let left = this_month.sub(Duration::days(-self.value as i64)).and_hms(0, 0, 0).timestamp_millis();
                let right = next_month.sub(Duration::days(-self.value as i64)).and_hms(0, 0, 0).timestamp_millis();
                (left, right)
            }
        };
        Ok(rtn)
    }

    fn get_year(&self, nd: &NaiveDateTime) -> Result<(i64, i64)> {
        if self.value > 199 || self.value < -200 {
            return Err(NatureError::LogicalError("value must in [-7,6]".to_string()));
        }
        let year_begin = Local.ymd(nd.year(), 1, 1);
        let today = Local.ymd(nd.year(), nd.month(), nd.day());
        let offset = today.sub(year_begin).num_days() as i16;
        let mut value = self.value;
        if value < 0 {
            value += 365
        }
        let diff_day = if value <= offset {
            offset - value
        } else {
            365 - value + offset
        };
        let left = today.sub(Duration::days(diff_day as i64)).and_hms(0, 0, 0);
        let right = if self.value >= 0 {
            Local.ymd(left.year() + 1, left.month(), left.day())
        } else {
            let end = Local.ymd(left.year() + 2, 1, 1);
            end.sub(Duration::days(-self.value as i64))
        };
        Ok((left.timestamp_millis(), right.and_hms(0, 0, 0).timestamp_millis()))
    }
}

fn get_next_month(nd: &NaiveDate) -> Date<Local> {
    if nd.month() < 12 {
        Local.ymd(nd.year(), nd.month() + 1, 1)
    } else {
        Local.ymd(nd.year() + 1, 1, 1)
    }
}

fn get_previous_month(nd: &NaiveDate) -> Date<Local> {
    if nd.month() > 1 {
        Local.ymd(nd.year(), nd.month() - 1, 1)
    } else {
        Local.ymd(nd.year() - 1, 12, 1)
    }
}


/// generate a timer para
pub fn time_range(input: &ConverterParameter) -> ConverterReturned {
    // get setting
    let cfg = match serde_json::from_str::<Setting>(&input.cfg) {
        Ok(cfg) => cfg,
        Err(err) => {
            warn!("error setting: {}", &input.cfg);
            return ConverterReturned::LogicalError(err.to_string());
        }
    };
    let result = match cfg.get_time(input.from.create_time) {
        Ok(rtn) => rtn,
        Err(err) => return ConverterReturned::LogicalError(err.to_string())
    };
    let mut instance = Instance::default();
    instance.para = format!("{}{}{}", result.0, *DEFAULT_PARA_SEPARATOR, result.1);
    ConverterReturned::Instances(vec![instance])
}

fn is_s(cmp: &str) -> bool {
    cmp.eq("s")
}

fn string_s() -> String {
    "s".to_string()
}

#[cfg(test)]
mod timer_setting_test {
    use super::*;

    #[test]
    fn test() {
        let mut setting = Setting {
            unit: "s".to_string(),
            value: 0,
        };
        let rtn = serde_json::to_string(&setting).unwrap();
        assert_eq!(rtn, "{}");
        let rtn: Setting = serde_json::from_str("{}").unwrap();
        assert_eq!(rtn, setting);
        setting.unit = "a".to_string();
        setting.value = 100;
        let rtn = serde_json::to_string(&setting).unwrap();
        let json = r#"{"unit":"a","value":100}"#;
        assert_eq!(rtn, json);
        let rtn: Setting = serde_json::from_str(json).unwrap();
        assert_eq!(rtn, setting);
    }

    #[test]
    fn second_test() {
        let time = Local.ymd(2020, 5, 1).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "s".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(18, 36, 23).timestamp_millis(),
            Local.ymd(2020, 5, 1).and_hms(18, 36, 24).timestamp_millis(),
        );
        assert_eq!(rtn, cmp);
        // test interval
        setting.value = 15;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(18, 36, 15).timestamp_millis(),
            Local.ymd(2020, 5, 1).and_hms(18, 36, 16).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
    }

    #[test]
    fn minute_test() {
        let time = Local.ymd(2020, 5, 1).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "m".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(18, 36, 0).timestamp_millis(),
            Local.ymd(2020, 5, 1).and_hms(18, 37, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // test interval
        setting.value = 10;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(18, 30, 0).timestamp_millis(),
            Local.ymd(2020, 5, 1).and_hms(18, 31, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
    }

    #[test]
    fn hour_test() {
        let time = Local.ymd(2020, 5, 1).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "h".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(18, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 1).and_hms(19, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // test interval
        setting.value = 4;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(16, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 1).and_hms(17, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
    }

    #[test]
    fn day_test() {
        let time = Local.ymd(2020, 5, 1).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "d".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 2).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // test interval
        setting.value = 3;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 4, 29).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 4, 30).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
    }

    #[test]
    fn week_test() {
        // the `value` is positive and before the real time
        let time = Local.ymd(2020, 5, 1).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "w".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 4, 27).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 4).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is positive and after the real time
        setting.value = 6;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 4, 26).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 3).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is negative and before the real time
        setting.value = -7;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 4, 27).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 4).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is negative and after the real time
        setting.value = -1;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 4, 26).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 3).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // range test
        setting.value = 7;
        let rtn = setting.get_time(time);
        assert!(rtn.is_err());
        setting.value = -8;
        let rtn = setting.get_time(time);
        assert!(rtn.is_err());
    }

    #[test]
    fn month_test() {
        // the `value` is positive and before the real time
        let time = Local.ymd(2020, 5, 28).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "M".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 1).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 6, 1).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is positive and after the real time
        setting.value = 6;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 7).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 6, 7).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is negative and before the real time
        setting.value = -5;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 5, 27).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 6, 26).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is negative and after the real time
        setting.value = -1;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 4, 30).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 5, 31).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // range test
        setting.value = 19;
        let rtn = setting.get_time(time);
        assert!(rtn.is_ok());
        setting.value = 20;
        let rtn = setting.get_time(time);
        assert!(rtn.is_err());
        setting.value = -20;
        let rtn = setting.get_time(time);
        assert!(rtn.is_ok());
        setting.value = -21;
        let rtn = setting.get_time(time);
        assert!(rtn.is_err());
    }

    #[test]
    fn year_test() {
        // the `value` is positive and before the real time
        let time = Local.ymd(2020, 11, 21).and_hms_milli(18, 36, 23, 123).timestamp_millis();
        let mut setting = Setting {
            unit: "y".to_string(),
            value: 0,
        };
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 1, 1).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2021, 1, 1).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is positive and after the real time
        setting.value = 6;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 1, 7).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2021, 1, 7).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is negative and before the real time
        setting.value = -1;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2019, 12, 31).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2020, 12, 31).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // the `value` is negative and after the real time
        setting.value = -50;
        let rtn = setting.get_time(time).unwrap();
        let cmp = (
            Local.ymd(2020, 11, 11).and_hms(0, 0, 0).timestamp_millis(),
            Local.ymd(2021, 11, 12).and_hms(0, 0, 0).timestamp_millis()
        );
        assert_eq!(rtn, cmp);
        // range test
        setting.value = 199;
        let rtn = setting.get_time(time);
        assert!(rtn.is_ok());
        setting.value = 200;
        let rtn = setting.get_time(time);
        assert!(rtn.is_err());
        setting.value = -200;
        let rtn = setting.get_time(time);
        assert!(rtn.is_ok());
        setting.value = -201;
        let rtn = setting.get_time(time);
        assert!(rtn.is_err());
    }
}

