#![forbid(unsafe_code)]

const RULE_WIDTH: usize = 78;

pub(crate) fn rule() -> String {
    "─".repeat(RULE_WIDTH)
}

pub(crate) fn human_bytes(bytes: usize) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.2} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn human_duration_us(microseconds: f64) -> String {
    if microseconds >= 1_000_000.0 {
        format!("{:.2} s", microseconds / 1_000_000.0)
    } else if microseconds >= 1_000.0 {
        format!("{:.2} ms", microseconds / 1_000.0)
    } else {
        format!("{microseconds:.1} µs")
    }
}

pub(crate) fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    let mut output = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    output.push('…');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_for_humans() {
        assert_eq!(human_bytes(21), "21 B");
        assert_eq!(human_bytes(30_008), "29.3 KiB");
        assert_eq!(human_bytes(1_048_576), "1.00 MiB");
    }

    #[test]
    fn formats_duration_for_humans() {
        assert_eq!(human_duration_us(866.7), "866.7 µs");
        assert_eq!(human_duration_us(1_329.4), "1.33 ms");
        assert_eq!(human_duration_us(1_500_000.0), "1.50 s");
    }

    #[test]
    fn truncation_is_unicode_safe() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("µµµ", 4), "µµµ");
    }
}
