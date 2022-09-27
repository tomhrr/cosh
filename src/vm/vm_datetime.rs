use std::cell::RefCell;
use std::convert::TryFrom;
use std::rc::Rc;
use std::str::FromStr;

use chrono::{NaiveDateTime, DateTime, Utc, Duration, TimeZone};
use chronoutil::RelativeDuration;

use vm::*;

impl VM {
    /// Returns the current time as a DateTime object, offset at UTC.
    pub fn core_now(&mut self) -> i32 {
        let date = chrono::offset::Utc::now();
        let newdate = date.with_timezone(&self.utc_tz);
        self.stack.push(Value::DateTimeNT(newdate));
        return 1;
    }

    /// Returns the current time as a DateTime object, offset at the
    /// local time zone.
    pub fn core_lcnow(&mut self) -> i32 {
        let date = chrono::offset::Utc::now();
        let newdate = date.with_timezone(&self.local_tz);
        self.stack.push(Value::DateTimeNT(newdate));
        return 1;
    }

    /// Takes a DateTime object and returns the epoch time that
    /// corresponds to that object.
    pub fn core_to_epoch(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("to-epoch requires one argument");
            return 0;
        }

        let dt_rr = self.stack.pop().unwrap();
        match dt_rr {
            Value::DateTimeNT(dt) => {
                let epoch = dt.timestamp();
                let epoch32 = i32::try_from(epoch).unwrap();
                self.stack.push(Value::Int(epoch32));
                return 1;
            },
            Value::DateTimeOT(dt) => {
                let epoch = dt.timestamp();
                let epoch32 = i32::try_from(epoch).unwrap();
                self.stack.push(Value::Int(epoch32));
                return 1;
            },
            _ => {
                self.print_error("unexpected argument");
                return 0;
            }
        }
    }

    /// Takes the epoch time (i.e. the number of seconds that have
    /// elapsed since 1970-01-01 00:00:00 UTC) and returns a DateTime
    /// object (offset at UTC) that corresponds to that time.
    pub fn core_from_epoch(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("from-epoch requires one argument");
            return 0;
        }

        let epoch_rr = self.stack.pop().unwrap();
        let epoch_int_opt = epoch_rr.to_int();
        match epoch_int_opt {
            Some(epoch_int) => {
                let epoch64 = i64::try_from(epoch_int).unwrap();
                let naive = NaiveDateTime::from_timestamp(epoch64, 0);
                let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
                let newdate = datetime.with_timezone(&self.utc_tz);
                self.stack.push(Value::DateTimeNT(newdate));
                return 1;
            }
            _ => {
                self.print_error("unexpected argument");
                return 0;
            }
        }
    }

    /// Takes a DateTime object and a named timezone (per the tz
    /// database) and returns a new DateTime object offset at that
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
		let tzr = chrono_tz::Tz::from_str(&s);
                match tzr {
                    Ok(tz) => {
			let newdate = dt.with_timezone(&tz);
                        self.stack.push(Value::DateTimeNT(newdate));
                        return 1;
                    },
                    _ => {
                        self.print_error("unknown timezone");
                        return 0;
                    }
                }
            },
            (Value::DateTimeOT(dt), Some(s)) => {
		let tzr = chrono_tz::Tz::from_str(&s);
                match tzr {
                    Ok(tz) => {
			let newdate = dt.with_timezone(&tz);
                        self.stack.push(Value::DateTimeNT(newdate));
                        return 1;
                    },
                    _ => {
                        self.print_error("unknown timezone");
                        return 0;
                    }
                }
            },
            (_, _) => {
		self.print_error("unexpected arguments");
                return 0;
            }
        }
    }

    /// Takes a DateTime object, a period (one of years, months, days,
    /// minutes, hours, or seconds) and a count as its arguments.
    /// Adds the specified number of periods to the DateTime object
    /// and returns the result as a new DateTime object.
    pub fn core_addtime(&mut self) -> i32 {
	if self.stack.len() < 3 {
            self.print_error("+time requires three arguments");
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
            },
            (Some("months"), Some(n)) => {
                rdur = Some(RelativeDuration::months(n));
            },
            (Some("days"), Some(n)) => {
                dur = Some(Duration::days(i64::try_from(n).unwrap()));
            },
            (Some("hours"), Some(n)) => {
                dur = Some(Duration::hours(i64::try_from(n).unwrap()));
            },
            (Some("minutes"), Some(n)) => {
                dur = Some(Duration::minutes(i64::try_from(n).unwrap()));
            },
            (Some("seconds"), Some(n)) => {
                dur = Some(Duration::seconds(i64::try_from(n).unwrap()));
            },
            (_, _) => {
		self.print_error("unexpected arguments");
                return 0;
            }
        }

        match (dt_rr, dur, rdur) {
            (Value::DateTimeNT(dt), Some(d), _) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeNT(ndt));
                return 1;
            },
            (Value::DateTimeNT(dt), _, Some(d)) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeNT(ndt));
                return 1;
            },
            (Value::DateTimeOT(dt), Some(d), _) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeOT(ndt));
                return 1;
            },
            (Value::DateTimeOT(dt), _, Some(d)) => {
                let ndt = dt + d;
                self.stack.push(Value::DateTimeOT(ndt));
                return 1;
            },
            _ => {
                self.print_error("unexpected arguments");
                return 0;
            }
        }
    }

    /// Takes a DateTime object, a period (one of years, months, days,
    /// minutes, hours, or seconds) and a count as its arguments.
    /// Subtracts the specified number of periods to the DateTime
    /// object and returns the result as a new DateTime object.
    pub fn core_subtime(&mut self) -> i32 {
	if self.stack.len() < 3 {
            self.print_error("-time requires three arguments");
            return 0;
        }

        let num_rr = self.stack.pop().unwrap();
        let num_int_opt = num_rr.to_int();

        match num_int_opt {
            Some(n) => {
                let n2 = n * -1;
                self.stack.push(Value::Int(n2));
                return self.core_addtime();
            },
            _ => {
                self.print_error("unexpected arguments");
                return 0;
            }
        }
    }

    /// Takes a DateTime object and a strftime pattern as its
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
                let ss = dt.format(s);
                self.stack.push(Value::String(Rc::new(RefCell::new(StringPair::new(ss.to_string(), None)))));
                return 1;
            },
            (Value::DateTimeOT(dt), Some(s)) => {
                let ss = dt.format(s);
                self.stack.push(Value::String(Rc::new(RefCell::new(StringPair::new(ss.to_string(), None)))));
                return 1;
            },
            (_, _) => {
		self.print_error("unexpected arguments");
                return 0;
            }
        }
    }

    /// Takes a datetime string and a strftime pattern as its
    /// arguments.  Returns the parsed datetime string as a DateTime
    /// object.
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
                if pat.contains("%z") || pat.contains("%Z") {
                    let dt_res = DateTime::parse_from_str(&st, &pat);
                    match dt_res {
                        Ok(dt) => {
                            self.stack.push(
                                Value::DateTimeOT(dt)
                            );
                            return 1;
                        },
                        _ => {
                            self.print_error("unable to parse datetime");
                            return 0;
                        }
                    }
                } else {
                    let dt_res = NaiveDateTime::parse_from_str(&st, &pat);
                    match dt_res {
                        Ok(naive) => {
                            let dt: DateTime<Utc> = DateTime::from_utc(naive, Utc);
                            self.stack.push(
                                Value::DateTimeNT(dt.with_timezone(&self.utc_tz))
                            );
                            return 1;
                        },
                        _ => {
                            self.print_error("unable to parse datetime");
                            return 0;
                        }
                    }
                }
            }
            (_, _) => {
		self.print_error("unexpected arguments");
                return 0;
            }
        }
    }

    /// Takes a datetime string, a strftime pattern, and a named
    /// timezone (per the tz database) as its arguments.  Returns the
    /// parsed datetime string as a DateTime object.
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
		let tzr = chrono_tz::Tz::from_str(&tzs);
                match tzr {
                    Ok(tz) => {
                        let dt_res = NaiveDateTime::parse_from_str(&st, &pat);
                        match dt_res {
                            Ok(naive) => {
                                self.stack.push(
                                    Value::DateTimeNT(tz.from_local_datetime(&naive).unwrap())
                                );
                                return 1;
                            },
                            _ => {
                                self.print_error("unable to parse datetime");
                                return 0;
                            }
                        }
                    }
                    _ => {
                        self.print_error("unknown timezone");
                        return 0;
                    }
                }
            }
            (_, _, _) => {
		self.print_error("unexpected arguments");
                return 0;
            }
        }
    }
}
