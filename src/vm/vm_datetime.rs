use std::convert::TryFrom;
use std::fmt::Write;
use std::str::FromStr;

use chrono::format::{parse, Parsed, StrftimeItems};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use chronoutil::RelativeDuration;

use crate::vm::*;

impl VM {
    /// Returns the current time as a date-time object, offset at UTC.
    pub fn core_now(&mut self) -> i32 {
        let date = chrono::offset::Utc::now();
        let newdate = date.with_timezone(&self.utc_tz);
        self.stack.push(Value::DateTimeNT(newdate));
        1
    }

    /// Returns the current time as a date-time object, offset at the
    /// local time zone.
    pub fn core_date(&mut self) -> i32 {
        let date = chrono::offset::Utc::now();
        let newdate = date.with_timezone(&self.local_tz);
        self.stack.push(Value::DateTimeNT(newdate));
        1
    }

    /// Takes a date-time object and returns the epoch time that
    /// corresponds to that object.
    pub fn core_to_epoch(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("to-epoch requires one argument");
            return 0;
        }

        let dt_rr = self.stack.pop().unwrap();
        match dt_rr {
            Value::DateTimeNT(dt) => {
                let epoch = dt.timestamp();
                let epoch32 = i32::try_from(epoch).unwrap();
                self.stack.push(Value::Int(epoch32));
                1
            }
            Value::DateTimeOT(dt) => {
                let epoch = dt.timestamp();
                let epoch32 = i32::try_from(epoch).unwrap();
                self.stack.push(Value::Int(epoch32));
                1
            }
            _ => {
                self.print_error("to-epoch argument must be date-time object");
                0
            }
        }
    }

    /// Takes the epoch time (i.e. the number of seconds that have
    /// elapsed since 1970-01-01 00:00:00 UTC) and returns a date-time
    /// object (offset at UTC) that corresponds to that time.
    pub fn core_from_epoch(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("from-epoch requires one argument");
            return 0;
        }

        let epoch_rr = self.stack.pop().unwrap();
        let epoch_int_opt = epoch_rr.to_int();
        match epoch_int_opt {
            Some(epoch_int) => {
                let epoch64 = i64::try_from(epoch_int).unwrap();
                let naive = NaiveDateTime::from_timestamp_opt(epoch64, 0).unwrap();
                let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);
                let newdate = datetime.with_timezone(&self.utc_tz);
                self.stack.push(Value::DateTimeNT(newdate));
                1
            }
            _ => {
                self.print_error("from-epoch argument must be integer");
                0
            }
        }
    }

    /// Takes a date-time object and a named timezone (per the tz
    /// database) and returns a new date-time object offset at that
    /// timezone.
    pub fn core_set_tz(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("set-tz requires two arguments");
            return 0;
        }

        let tz_rr = self.stack.pop().unwrap();
        let tz_opt: Option<&str>;
        to_str!(tz_rr, tz_opt);

        let dt_rr = self.stack.pop().unwrap();

        match (dt_rr, tz_opt) {
            (Value::DateTimeNT(dt), Some(s)) => {
                let tzr = chrono_tz::Tz::from_str(s);
                match tzr {
                    Ok(tz) => {
                        let newdate = dt.with_timezone(&tz);
                        self.stack.push(Value::DateTimeNT(newdate));
                        1
                    }
                    _ => {
                        self.print_error("second set-tz argument must be valid timezone");
                        0
                    }
                }
            }
            (Value::DateTimeOT(dt), Some(s)) => {
                let tzr = chrono_tz::Tz::from_str(s);
                match tzr {
                    Ok(tz) => {
                        let newdate = dt.with_timezone(&tz);
                        self.stack.push(Value::DateTimeNT(newdate));
                        1
                    }
                    _ => {
                        self.print_error("second set-tz argument must be valid timezone");
                        0
                    }
                }
            }
            (_, _) => {
                self.print_error("first set-tz argument must be date-time object");
                0
            }
        }
    }

    /// The internal time-modification function.  Takes a function name
    /// argument that is used only in error messages, so that this can
    /// be used by both +time and -time.
    fn addtime(&mut self, fn_name: &str) -> i32 {
        if self.stack.len() < 3 {
            let err_str = format!("{} requires three arguments", fn_name);
            self.print_error(&err_str);
            return 0;
        }

        let num_rr = self.stack.pop().unwrap();
        let num_int_opt = num_rr.to_int();

        let period_rr = self.stack.pop().unwrap();
        let period_opt: Option<&str>;
        to_str!(period_rr, period_opt);

        let dt_rr = self.stack.pop().unwrap();
        let mut rdur = None;
        let mut dur = None;

        match (period_opt, num_int_opt) {
            (Some("years"), Some(n)) => {
                rdur = Some(RelativeDuration::years(n));
            }
            (Some("months"), Some(n)) => {
                rdur = Some(RelativeDuration::months(n));
            }
            (Some("days"), Some(n)) => {
                dur = Some(Duration::days(i64::try_from(n).unwrap()));
            }
            (Some("hours"), Some(n)) => {
                dur = Some(Duration::hours(i64::try_from(n).unwrap()));
            }
            (Some("minutes"), Some(n)) => {
                dur = Some(Duration::minutes(i64::try_from(n).unwrap()));
            }
            (Some("seconds"), Some(n)) => {
                dur = Some(Duration::seconds(i64::try_from(n).unwrap()));
            }
            (
                Some("years") | Some("months") | Some("days") | Some("hours") | Some("minutes")
                | Some("seconds"),
                _,
            ) => {
                let err_str = format!("third {} argument must be integer", fn_name);
                self.print_error(&err_str);
                return 0;
            }
            (..) => {
                let err_str = format!("second {} argument must be time unit", fn_name);
                self.print_error(&err_str);
                return 0;
            }
        }

        match (dt_rr, dur, rdur) {
            (Value::DateTimeNT(dt), Some(d), _) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeNT(ndt));
                1
            }
            (Value::DateTimeNT(dt), _, Some(d)) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeNT(ndt));
                1
            }
            (Value::DateTimeOT(dt), Some(d), _) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeOT(ndt));
                1
            }
            (Value::DateTimeOT(dt), _, Some(d)) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeOT(ndt));
                1
            }
            (Value::DateTimeNT(_) | Value::DateTimeOT(_), _, _) => {
                let err_str = format!("second {} argument must be time unit", fn_name);
                self.print_error(&err_str);
                0
            }
            (..) => {
                let err_str = format!("second {} argument must be date-time object", fn_name);
                self.print_error(&err_str);
                0
            }
        }
    }

    /// Takes a date-time object, a period (one of years, months, days,
    /// minutes, hours, or seconds) and a count as its arguments.
    /// Adds the specified number of periods to the date-time object
    /// and returns the result as a new date-time object.
    pub fn core_addtime(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("+time requires three arguments");
            return 0;
        }

        self.addtime("+time")
    }

    /// Takes a date-time object, a period (one of years, months, days,
    /// minutes, hours, or seconds) and a count as its arguments.
    /// Subtracts the specified number of periods to the date-time
    /// object and returns the result as a new date-time object.
    pub fn core_subtime(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("-time requires three arguments");
            return 0;
        }

        let num_rr = self.stack.pop().unwrap();
        let num_int_opt = num_rr.to_int();

        match num_int_opt {
            Some(n) => {
                self.stack.push(Value::Int(-n));
                self.addtime("-time")
            }
            _ => {
                self.print_error("third -time argument must be integer");
                0
            }
        }
    }

    /// Takes a date-time object and a strftime pattern as its
    /// arguments.  Returns the stringification of the date per the
    /// pattern.
    pub fn core_strftime(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("strftime requires two arguments");
            return 0;
        }

        let pat_rr = self.stack.pop().unwrap();
        let pat_opt: Option<&str>;
        to_str!(pat_rr, pat_opt);

        let dt_rr = self.stack.pop().unwrap();

        match (dt_rr, pat_opt) {
            (Value::DateTimeNT(dt), Some(s)) => {
                let mut buffer = String::new();
                let res = write!(buffer, "{}", dt.format(s));
                match res {
                    Ok(_) => {
                        self.stack.push(new_string_value(buffer));
                        1
                    }
                    Err(_) => {
                        self.print_error("second strftime argument is invalid");
                        0
                    }
                }
            }
            (Value::DateTimeOT(dt), Some(s)) => {
                let ss = dt.format(s);
                self.stack.push(new_string_value(ss.to_string()));
                1
            }
            (_, Some(_)) => {
                self.print_error("first strftime argument must be date-time object");
                0
            }
            (..) => {
                self.print_error("second strftime argument must be string");
                0
            }
        }
    }

    /// The internal strptime function, used by both core_strptime and
    /// core_strptimez.
    fn strptime(&mut self, pattern: &str, value: &str) -> Option<Parsed> {
        let mut parsed = Parsed::new();
        let si = StrftimeItems::new(pattern);
        let res = parse(&mut parsed, value, si);
        match res {
            Ok(_) => {
                if parsed.year.is_none() {
                    parsed.set_year(1970).unwrap();
                }
                if parsed.month.is_none() {
                    parsed.set_month(1).unwrap();
                }
                if parsed.day.is_none() {
                    parsed.set_day(1).unwrap();
                }
                if parsed.hour_div_12.is_none() {
                    parsed.set_hour(0).unwrap();
                }
                if parsed.hour_mod_12.is_none() {
                    parsed.set_hour(0).unwrap();
                }
                if parsed.minute.is_none() {
                    parsed.set_minute(0).unwrap();
                }
                if parsed.second.is_none() {
                    parsed.set_second(0).unwrap();
                }
                if parsed.offset.is_none() {
                    parsed.set_offset(0).unwrap();
                }
                Some(parsed)
            }
            Err(e) => {
                let err_str = format!("unable to parse date-time string: {}", e);
                self.print_error(&err_str);
                None
            }
        }
    }

    /// Takes a datetime string and a strftime pattern as its
    /// arguments.  Returns the parsed datetime string as a date-time
    /// object.  The parsed datetime defaults to 1970-01-01 00:00:00,
    /// with components got via the strftime pattern applied on top of
    /// the default.
    pub fn core_strptime(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("strptime requires two arguments");
            return 0;
        }

        let pat_rr = self.stack.pop().unwrap();
        let pat_opt: Option<&str>;
        to_str!(pat_rr, pat_opt);

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match (str_opt, pat_opt) {
            (Some(st), Some(pat)) => {
                let parsed_opt = self.strptime(pat, st);
                match parsed_opt {
                    Some(parsed) => {
                        let dt_res = parsed.to_datetime().unwrap();
                        self.stack.push(Value::DateTimeOT(dt_res));
                        1
                    }
                    _ => 0,
                }
            }
            (Some(_), _) => {
                self.print_error("second strptime argument must be a string");
                0
            }
            (..) => {
                self.print_error("first strptime argument must be a string");
                0
            }
        }
    }

    /// Takes a datetime string, a strftime pattern, and a named
    /// timezone (per the tz database) as its arguments.  Returns the
    /// parsed datetime string as a date-time object.  The parsed
    /// datetime defaults to 1970-01-01 00:00:00, with components got
    /// via the strftime pattern applied on top of the default.
    pub fn core_strptimez(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("strptimez requires three arguments");
            return 0;
        }

        let tz_rr = self.stack.pop().unwrap();
        let tz_opt: Option<&str>;
        to_str!(tz_rr, tz_opt);

        let pat_rr = self.stack.pop().unwrap();
        let pat_opt: Option<&str>;
        to_str!(pat_rr, pat_opt);

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match (str_opt, pat_opt, tz_opt) {
            (Some(st), Some(pat), Some(tzs)) => {
                let tzr = chrono_tz::Tz::from_str(tzs);
                match tzr {
                    Ok(tz) => {
                        let parsed_opt = self.strptime(pat, st);
                        match parsed_opt {
                            Some(parsed) => {
                                let dt_res = parsed
                                    .to_naive_date()
                                    .unwrap()
                                    .and_time(parsed.to_naive_time().unwrap());
                                self.stack.push(Value::DateTimeNT(
                                    tz.from_local_datetime(&dt_res).unwrap(),
                                ));
                                1
                            }
                            _ => 0,
                        }
                    }
                    _ => {
                        self.print_error("third strptimez argument must be valid timezone");
                        0
                    }
                }
            }
            (Some(_), Some(_), _) => {
                self.print_error("third strptimez argument must be string");
                0
            }
            (Some(_), _, _) => {
                self.print_error("second strptimez argument must be string");
                0
            }
            (..) => {
                self.print_error("first strptimez argument must be string");
                0
            }
        }
    }
}
