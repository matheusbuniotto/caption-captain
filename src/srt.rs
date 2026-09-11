/// A single subtitle cue: a time range and its text.
pub struct Cue {
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
}

/// Renders cues as a standard numbered `.srt` document.
pub fn format_srt(cues: &[Cue]) -> String {
    let mut out = String::new();
    for (i, cue) in cues.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(cue.start_ms),
            format_timestamp(cue.end_ms)
        ));
        out.push_str(cue.text.trim());
        out.push_str("\n\n");
    }
    out
}

fn format_timestamp(total_ms: u32) -> String {
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_single_cue() {
        let cues = vec![Cue {
            start_ms: 0,
            end_ms: 1500,
            text: "Hello there".to_string(),
        }];
        assert_eq!(
            format_srt(&cues),
            "1\n00:00:00,000 --> 00:00:01,500\nHello there\n\n"
        );
    }

    #[test]
    fn numbers_cues_sequentially_and_trims_text() {
        let cues = vec![
            Cue {
                start_ms: 0,
                end_ms: 1000,
                text: "  First  ".to_string(),
            },
            Cue {
                start_ms: 1000,
                end_ms: 2000,
                text: "Second".to_string(),
            },
        ];
        let srt = format_srt(&cues);
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,000\nFirst\n\n"));
        assert!(srt.contains("2\n00:00:01,000 --> 00:00:02,000\nSecond\n\n"));
    }

    #[test]
    fn formats_timestamps_past_an_hour() {
        let cues = vec![Cue {
            start_ms: 3_661_234,
            end_ms: 3_662_000,
            text: "late".to_string(),
        }];
        assert!(format_srt(&cues).contains("01:01:01,234 --> 01:01:02,000"));
    }
}
