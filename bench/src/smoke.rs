//! Config edits the smoke test makes to a running bar, and what it reads back.

/// The `"name"` of every output in `swaymsg -t get_outputs -r` JSON, in order.
/// Output names never contain quotes or escapes, so a scan is enough.
pub fn output_names(json: &str) -> Vec<String> {
    json.split("\"name\":")
        .skip(1)
        .filter_map(|rest| {
            let rest = rest.trim_start().strip_prefix('"')?;
            Some(rest[..rest.find('"')?].to_owned())
        })
        .collect()
}

/// The same config with the bar moved to the bottom edge. Keeps every other
/// line, so the edit is what a user saving their file would do.
pub fn move_to_bottom(config: &str) -> String {
    const LINE: &str = "position = \"bottom\"";
    let mut lines: Vec<String> = config.lines().map(str::to_owned).collect();
    let mut table = String::new();
    let mut bar_header = None;
    for (i, line) in lines.iter_mut().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            table = trimmed.trim_matches(['[', ']']).trim().to_owned();
            if table == "bar" {
                bar_header = Some(i);
            }
        } else if table == "bar" && trimmed.split('=').next().is_some_and(|k| k.trim() == "position") {
            *line = LINE.to_owned();
            return lines.join("\n") + "\n";
        }
    }
    match bar_header {
        Some(i) => lines.insert(i + 1, LINE.to_owned()),
        None => {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.extend(["[bar]".to_owned(), LINE.to_owned()]);
        }
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_names_are_read_from_get_outputs() {
        let json = r#"[
  { "id": 3, "type": "output", "name": "HEADLESS-1", "make": "headless",
    "modes": [], "current_mode": { "width": 1920 } },
  {"id":7,"type":"output","name":"HEADLESS-2","make":"headless"}
]"#;
        assert_eq!(output_names(json), ["HEADLESS-1", "HEADLESS-2"]);
        assert!(output_names("[]").is_empty());
    }

    #[test]
    fn existing_position_is_rewritten_in_place() {
        let text = "[bar]\nposition = \"top\"   # edge\nheight = 30\n\n[clock]\nformat = \"%H:%M\"\n";
        assert_eq!(
            move_to_bottom(text),
            "[bar]\nposition = \"bottom\"\nheight = 30\n\n[clock]\nformat = \"%H:%M\"\n"
        );
    }

    #[test]
    fn bar_table_without_position_gets_one() {
        assert_eq!(move_to_bottom("[bar]\nheight = 30\n"), "[bar]\nposition = \"bottom\"\nheight = 30\n");
    }

    #[test]
    fn config_without_bar_table_gets_one() {
        assert_eq!(move_to_bottom(""), "[bar]\nposition = \"bottom\"\n");
        assert_eq!(
            move_to_bottom("[clock]\nformat = \"%H\"\n"),
            "[clock]\nformat = \"%H\"\n\n[bar]\nposition = \"bottom\"\n"
        );
    }
}
