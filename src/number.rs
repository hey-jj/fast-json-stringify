//! ECMAScript number formatting.
//!
//! JavaScript prints floats with `Number.prototype.toString`, which `JSON.stringify`
//! reuses for finite numbers. Rust's default `f64` formatter disagrees in the
//! exponent range. `5e-7` prints as `0.0000005` and `1e21` prints with 22 digits.
//! To match byte for byte, this module reproduces the ECMA-262 ToString algorithm
//! (section 6.1.6.1.20) on top of the shortest round-trip digits from `ryu`.

/// Format a finite `f64` exactly as JavaScript's `String(n)` would.
///
/// Handles sign, the integer and fraction ranges, and the exponential form with
/// the same `n in (-6, 21]` thresholds ECMAScript uses. `-0.0` formats as `0`.
/// Non-finite inputs are not expected here. `NaN` returns `"NaN"` and the
/// infinities return `"Infinity"` / `"-Infinity"`, which callers handle earlier.
pub fn format_f64(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        // Covers both 0.0 and -0.0. JavaScript renders both as "0".
        return "0".to_string();
    }

    let negative = value.is_sign_negative();
    let magnitude = value.abs();

    // ryu gives the shortest decimal that round-trips. Parse it into the
    // significant digits `digits` and the power `point_exp` such that
    // value = digits * 10^(point_exp - len(digits)).
    let mut buf = ryu::Buffer::new();
    let ryu_str = buf.format_finite(magnitude);
    let (digits, point_exp) = decompose(ryu_str);

    let k = digits.len() as i32;
    let n = point_exp; // position of the decimal point relative to the first digit

    let body = ecma_body(&digits, k, n);
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

/// Build the digit-string body (no sign) from significant digits and the ECMA
/// `n`/`k` pair, applying the four ECMAScript range rules.
fn ecma_body(digits: &str, k: i32, n: i32) -> String {
    if k <= n && n <= 21 {
        // Integer with trailing zeros: digits then (n - k) zeros.
        let mut out = String::with_capacity(n as usize);
        out.push_str(digits);
        for _ in 0..(n - k) {
            out.push('0');
        }
        out
    } else if 0 < n && n <= 21 {
        // Decimal point inside the digits.
        let (int_part, frac_part) = digits.split_at(n as usize);
        format!("{int_part}.{frac_part}")
    } else if -6 < n && n <= 0 {
        // Small magnitude: leading "0." then (-n) zeros then digits.
        let mut out = String::with_capacity((2 - n) as usize + digits.len());
        out.push_str("0.");
        for _ in 0..(-n) {
            out.push('0');
        }
        out.push_str(digits);
        out
    } else {
        // Exponential form.
        let exp = n - 1;
        let sign = if exp >= 0 { '+' } else { '-' };
        let exp_abs = exp.abs();
        if k == 1 {
            format!("{digits}e{sign}{exp_abs}")
        } else {
            let (first, rest) = digits.split_at(1);
            format!("{first}.{rest}e{sign}{exp_abs}")
        }
    }
}

/// Split a ryu-formatted finite string into significant digits and the decimal
/// point position `n` (value = digits * 10^(n - len(digits))).
///
/// ryu emits forms like `123`, `1.23`, `0.0005`, `1.5e-7`, `1e21`. This
/// normalizes all of them to a trailing-zero-free digit string plus `n`.
fn decompose(s: &str) -> (String, i32) {
    let (mantissa, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().unwrap_or(0)),
        None => (s, 0),
    };

    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };

    // All significant digits with the decimal point removed.
    let mut all = String::with_capacity(int_part.len() + frac_part.len());
    all.push_str(int_part);
    all.push_str(frac_part);

    // Point position before stripping: digits in int_part sit left of the point.
    // Fold the explicit exponent into it.
    let mut n = int_part.len() as i32 + exp;

    // Strip leading zeros, moving the point left for each one removed.
    let leading = all.len() - all.trim_start_matches('0').len();
    if leading > 0 {
        all.drain(..leading);
        n -= leading as i32;
    }

    // Strip trailing zeros, they do not change the point position.
    let trailing = all.len() - all.trim_end_matches('0').len();
    if trailing > 0 {
        all.truncate(all.len() - trailing);
    }

    if all.is_empty() {
        // Value was zero, but callers handle zero before reaching here.
        all.push('0');
        n = 1;
    }

    (all, n)
}

#[cfg(test)]
mod tests {
    use super::format_f64;

    #[test]
    fn matches_javascript_samples() {
        assert_eq!(format_f64(100.0), "100");
        assert_eq!(format_f64(1.33), "1.33");
        assert_eq!(format_f64(5e-7), "5e-7");
        assert_eq!(format_f64(-0.0), "0");
        assert_eq!(format_f64(1e21), "1e+21");
        assert_eq!(format_f64(0.1), "0.1");
        assert_eq!(format_f64(9007199254740991.0), "9007199254740991");
        assert_eq!(
            format_f64(1.7976931348623157e308),
            "1.7976931348623157e+308"
        );
        assert_eq!(format_f64(0.0001), "0.0001");
        assert_eq!(format_f64(0.0000001), "1e-7");
        assert_eq!(format_f64(42.42), "42.42");
        assert_eq!(format_f64(-45.05), "-45.05");
        assert_eq!(format_f64(123456789012345680.0), "123456789012345680");
        assert_eq!(format_f64(5e-324), "5e-324");
        assert_eq!(format_f64(1e-6), "0.000001");
        assert_eq!(format_f64(1e-7), "1e-7");
        assert_eq!(format_f64(1e20), "100000000000000000000");
        assert_eq!(format_f64(1234567890123456789.0), "1234567890123456800");
    }
}
