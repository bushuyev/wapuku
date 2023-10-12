use std::borrow::Cow;
use once_cell::sync::Lazy;
use regex::{Captures, Regex, Replacer};

pub struct FloatReformatter;

static RE:Lazy<Regex> =  Lazy::new(|| Regex::new(r"(?<num>\d+\.\d+)").expect("FloatReformatter regexp"));

impl Replacer for FloatReformatter {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        dst.push_str(format!("{:.2}", &caps["num"].parse::<f32>().unwrap_or(f32::NAN)).as_str());
    }
}

impl FloatReformatter {
    pub fn exec(txt:&str) -> Cow<str> {
        RE.replace_all(txt, FloatReformatter)
    }
}


pub fn val_or_na(v: &String) -> impl ToString + Sized + '_{
    if v.is_empty() {
        "n/a"
    } else {
        v
    }
}

#[cfg(test)]
mod util_tests {
    use crate::utils::FloatReformatter;

    #[test]
    fn test_fix_numeric_label() {
        assert_eq!(FloatReformatter::exec("(-inf, 0.0]"), "(-inf, 0.00]");
        assert_eq!(FloatReformatter::exec("(0.0, 0.4]"), "(0.00, 0.40]");
        assert_eq!(FloatReformatter::exec("(1.2000000000000002, 1.6]"), "(1.20, 1.60]");
        assert_eq!(FloatReformatter::exec("(2.4000000000000004, 2.8000000000000003]"), "(2.40, 2.80]");
    }
}