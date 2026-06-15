use crate::prelude::*;
use std::time::Instant;
#[expect(clippy::modulo_arithmetic)]
pub(crate) fn time_stamp() -> String {
  let now = now();
  let mut secs = now.as_secs() as i64;
  let millis = now.subsec_millis();
  let sec = (secs % 60) as i32;
  secs /= 60;
  let min = (secs % 60) as i32;
  secs /= 60;
  let hour = (secs % 24) as i32;
  let mut days = (secs / 24) as i32;
  let mut year = 1970;
  loop {
    let leap = is_leap(year);
    let days_in_year = if leap { 366 } else { 365 };
    if days >= days_in_year {
      days -= days_in_year;
      year += 1;
    } else {
      break;
    }
  }
  let mut month = 1;
  while let Some(md) = month_day(year, month)
    && days >= md
  {
    days -= md;
    month += 1;
  }
  let day = days + 1;
  format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z")
}
pub(crate) fn format_micros(start: Instant) -> String {
  let micros = start.elapsed().as_micros();
  let ms = micros / 1000;
  let frac = micros % 1000;
  format!("{ms}.{frac:03}ms")
}
fn is_leap(year: i32) -> bool {
  (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
fn month_day(year: i32, month: i32) -> Option<i32> {
  let leap = is_leap(year);
  match month {
    2 => Some(if leap { 29 } else { 28 }),
    4 | 6 | 9 | 11 => Some(30),
    1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
    _ => None,
  }
}
