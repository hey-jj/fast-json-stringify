//! The runtime value model fed to a compiled serializer.
//!
//! JavaScript hands the serializer plain JSON values plus a few host objects:
//! `Date`, `BigInt`, `RegExp`, and any object carrying a `toJSON` method. JSON
//! alone cannot express those, so [`Value`] adds them as explicit variants. A
//! plain [`serde_json::Value`] converts in with [`From`], so common inputs stay
//! ergonomic.

/// A JavaScript-shaped value handed to a compiled serializer.
///
/// The first six variants mirror JSON. `Date`, `BigInt`, and `RegExp` model the
/// host objects the serializer coerces. `Custom` carries a value that already
/// went through a `toJSON` hook, letting tests reproduce that behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// JSON `null`, or a deliberately absent value.
    Null,
    /// A boolean.
    Bool(bool),
    /// A finite or non-finite IEEE-754 double.
    Number(f64),
    /// An arbitrary-precision integer, the analog of a JavaScript `BigInt`.
    BigInt(i128),
    /// A UTF-8 string.
    String(String),
    /// An ordered list of values.
    Array(Vec<Value>),
    /// An insertion-ordered map of string keys to values.
    Object(Object),
    /// A `Date`, stored as milliseconds since the Unix epoch.
    Date(i64),
    /// A `RegExp`, stored as its source pattern.
    Regex(String),
    /// A value produced by an object's `toJSON` method.
    Custom(Box<Value>),
}

/// An insertion-ordered object. Key order is preserved because JSON output
/// order is observable and the tests assert on it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Object {
    entries: Vec<(String, Value)>,
}

impl Object {
    /// Create an empty object.
    pub fn new() -> Self {
        Object {
            entries: Vec::new(),
        }
    }

    /// Append or overwrite a key. Existing keys keep their position.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    /// Look up a key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Iterate entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// True when the object holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl FromIterator<(String, Value)> for Object {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let mut obj = Object::new();
        for (k, v) in iter {
            obj.insert(k, v);
        }
        obj
    }
}

impl Value {
    /// True when this value is `null`, matching `value === null` in the source.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Apply the `toJSON` unwrap used before object serialization. A [`Value::Custom`]
    /// or [`Value::Date`] yields its JSON projection, everything else is returned
    /// unchanged.
    pub fn unwrap_to_json(&self) -> std::borrow::Cow<'_, Value> {
        match self {
            Value::Custom(inner) => std::borrow::Cow::Borrowed(inner),
            Value::Date(_) => std::borrow::Cow::Owned(Value::String(self.date_iso().unwrap())),
            _ => std::borrow::Cow::Borrowed(self),
        }
    }

    /// True when the value would pass a `toJSON` presence check, that is a
    /// [`Value::Custom`] or [`Value::Date`].
    pub fn has_to_json(&self) -> bool {
        matches!(self, Value::Custom(_) | Value::Date(_))
    }

    /// Render a [`Value::Date`] as a full ISO 8601 UTC timestamp.
    pub fn date_iso(&self) -> Option<String> {
        if let Value::Date(ms) = self {
            Some(iso_from_millis(*ms))
        } else {
            None
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Value::Number(f)
                } else {
                    // arbitrary_precision can hold integers beyond f64. Keep them
                    // as BigInt when they fit i128, otherwise fall back to 0.
                    n.to_string()
                        .parse::<i128>()
                        .map(Value::BigInt)
                        .unwrap_or(Value::Number(0.0))
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(a) => Value::Array(a.into_iter().map(Value::from).collect()),
            serde_json::Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

/// Render epoch milliseconds as `YYYY-MM-DDTHH:mm:ss.sssZ`, matching
/// `Date.prototype.toISOString`.
pub(crate) fn iso_from_millis(ms: i64) -> String {
    let (y, mo, d, h, mi, s, milli) = civil_from_millis(ms);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z")
}

/// Render the date portion (`YYYY-MM-DD`) of epoch milliseconds.
pub(crate) fn date_slice_from_millis(ms: i64) -> String {
    let (y, mo, d, _, _, _, _) = civil_from_millis(ms);
    format!("{y:04}-{mo:02}-{d:02}")
}

/// Render the time portion (`HH:mm:ss`) of epoch milliseconds.
pub(crate) fn time_slice_from_millis(ms: i64) -> String {
    let (_, _, _, h, mi, s, _) = civil_from_millis(ms);
    format!("{h:02}:{mi:02}:{s:02}")
}

/// Convert epoch milliseconds to a UTC civil datetime. Uses Howard Hinnant's
/// days-from-civil inverse so no date library is needed and the math matches
/// the proleptic Gregorian calendar `Date` uses.
fn civil_from_millis(ms: i64) -> (i64, u32, u32, u32, u32, u32, u32) {
    let total_secs = ms.div_euclid(1000);
    let milli = ms.rem_euclid(1000) as u32;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);

    let h = (secs_of_day / 3600) as u32;
    let mi = ((secs_of_day % 3600) / 60) as u32;
    let s = (secs_of_day % 60) as u32;

    // civil_from_days, shifted so the era starts at 0000-03-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };

    (year, m, d, h, mi, s, milli)
}
