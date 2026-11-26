use std::borrow::Cow;

fn lookup_emoji(name: &str) -> Option<&'static str> {
    match name {
        // Status & Docs
        "rocket" => Some("🚀"),
        "white_check_mark" => Some("✅"),
        "heavy_check_mark" => Some("✔️"),
        "x" => Some("❌"),
        "warning" => Some("⚠️"),
        "bulb" => Some("💡"),
        "memo" => Some("📝"),
        "bug" => Some("🐛"),
        "sparkles" => Some("✨"),
        "fire" => Some("🔥"),
        "tada" => Some("🎉"),
        "wrench" => Some("🔧"),
        "hammer" => Some("🔨"),
        "hammer_and_wrench" => Some("🛠️"),
        "package" => Some("📦"),
        "lock" => Some("🔒"),
        "unlock" => Some("🔓"),
        "key" => Some("🔑"),
        "book" | "open_book" => Some("📖"),
        "books" => Some("📚"),
        "bookmark" => Some("🔖"),
        "link" => Some("🔗"),
        "pushpin" => Some("📌"),
        "round_pushpin" => Some("📍"),
        "star" => Some("⭐"),
        "star2" => Some("🌟"),
        "heart" => Some("❤️"),
        "blue_heart" => Some("💙"),
        "green_heart" => Some("💚"),
        "yellow_heart" => Some("💛"),
        "purple_heart" => Some("💜"),
        "+1" | "thumbsup" => Some("👍"),
        "-1" | "thumbsdown" => Some("👎"),
        "eyes" => Some("👀"),
        "clap" => Some("👏"),
        "zap" => Some("⚡"),
        "construction" => Some("🚧"),
        "question" => Some("❓"),
        "grey_question" => Some("❔"),
        "exclamation" => Some("❗"),
        "bangbang" => Some("‼️"),
        "information_source" => Some("ℹ️"),
        "no_entry" => Some("⛔"),
        "no_entry_sign" => Some("🚫"),
        "rotating_light" => Some("🚨"),
        "shield" => Some("🛡️"),
        "gear" => Some("⚙️"),
        "mag" => Some("🔍"),
        "mag_right" => Some("🔎"),

        // Arrows & Pointers
        "arrow_right" => Some("➡️"),
        "arrow_left" => Some("⬅️"),
        "arrow_up" => Some("⬆️"),
        "arrow_down" => Some("⬇️"),
        "arrow_upper_right" => Some("↗️"),
        "arrow_lower_right" => Some("↘️"),
        "arrows_counterclockwise" => Some("🔄"),
        "back" => Some("🔙"),
        "soon" => Some("🔜"),
        "top" => Some("🔝"),
        "point_right" => Some("👉"),
        "point_left" => Some("👈"),
        "point_up" => Some("☝️"),
        "point_down" => Some("👇"),

        // Files, Code & Tools
        "globe_with_meridians" => Some("🌐"),
        "computer" => Some("💻"),
        "desktop_computer" => Some("🖥️"),
        "keyboard" => Some("⌨️"),
        "file_folder" => Some("📁"),
        "open_file_folder" => Some("📂"),
        "page_facing_up" => Some("📄"),
        "page_with_curl" => Some("📃"),
        "clipboard" => Some("📋"),
        "card_index" => Some("📇"),
        "label" => Some("🏷️"),
        "paperclip" => Some("📎"),
        "scissors" => Some("✂️"),
        "pencil2" => Some("✏️"),
        "pen" | "fountain_pen" => Some("🖋️"),
        "wastebasket" => Some("🗑️"),
        "recycle" => Some("♻️"),
        "hourglass" => Some("⌛"),
        "hourglass_flowing_sand" => Some("⏳"),
        "stopwatch" => Some("⏱️"),
        "alarm_clock" => Some("⏰"),
        "calendar" => Some("📆"),
        "date" => Some("📅"),
        "chart_with_upwards_trend" => Some("📈"),
        "chart_with_downwards_trend" => Some("📉"),
        "bar_chart" => Some("📊"),
        "trophy" => Some("🏆"),
        "medal" | "sports_medal" => Some("🏅"),
        "1st_place_medal" => Some("🥇"),
        "100" => Some("💯"),
        "art" => Some("🎨"),

        // Communication & Faces
        "ok_hand" => Some("👌"),
        "wave" => Some("👋"),
        "raised_hands" => Some("🙌"),
        "pray" => Some("🙏"),
        "muscle" => Some("💪"),
        "brain" => Some("🧠"),
        "robot" => Some("🤖"),
        "thinking" => Some("🤔"),
        "smile" => Some("😄"),
        "grinning" => Some("😀"),
        "blush" => Some("😊"),
        "wink" => Some("😉"),
        "sunglasses" => Some("😎"),
        "joy" => Some("😂"),
        "sob" => Some("😭"),
        "nerd_face" => Some("🤓"),
        "confetti_ball" => Some("🎊"),
        "gift" => Some("🎁"),
        "bell" => Some("🔔"),
        "loudspeaker" => Some("📢"),
        "mega" => Some("📣"),
        "speech_balloon" => Some("💬"),
        "thought_balloon" => Some("💭"),
        "email" | "envelope" => Some("✉️"),
        "inbox_tray" => Some("📥"),
        "outbox_tray" => Some("📤"),

        // Badges & Symbols
        "checkered_flag" => Some("🏁"),
        "triangular_flag_on_post" => Some("🚩"),
        "red_circle" => Some("🔴"),
        "orange_circle" => Some("🟠"),
        "yellow_circle" => Some("🟡"),
        "green_circle" => Some("🟢"),
        "blue_circle" => Some("🔵"),
        "purple_circle" => Some("🟣"),
        "white_circle" => Some("⚪"),
        "black_circle" => Some("⚫"),
        "coffee" => Some("☕"),
        "sunny" => Some("☀️"),
        "cloud" => Some("☁️"),
        "snowflake" => Some("❄️"),
        "rainbow" => Some("🌈"),
        "crescent_moon" => Some("🌙"),
        "new" => Some("🆕"),
        "free" => Some("🆓"),
        "sos" => Some("🆘"),
        "copyright" => Some("©️"),
        "registered" => Some("®️"),
        "tm" => Some("™️"),
        _ => None,
    }
}

pub fn replace_emoji_shortcodes(input: &str) -> Cow<'_, str> {
    if !input.contains(':') {
        return Cow::Borrowed(input);
    }

    let bytes = input.as_bytes();
    let mut out: Option<String> = None;
    let mut last_copied = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b':' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && j - start <= 36 {
                let b = bytes[j];
                if b == b':' {
                    break;
                }
                if !(b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'+' | b'-'))
                {
                    break;
                }
                j += 1;
            }

            if j > start
                && j < bytes.len()
                && bytes[j] == b':'
                && let Some(emoji) = lookup_emoji(&input[start..j])
            {
                let buf = out.get_or_insert_with(|| String::with_capacity(input.len()));
                buf.push_str(&input[last_copied..i]);
                buf.push_str(emoji);
                last_copied = j + 1;
                i = last_copied;
                continue;
            }
        }
        i += 1;
    }

    match out {
        Some(mut s) => {
            if last_copied < input.len() {
                s.push_str(&input[last_copied..]);
            }
            Cow::Owned(s)
        }
        None => Cow::Borrowed(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_known_github_shortcodes_and_preserves_unknown() {
        assert_eq!(
            replace_emoji_shortcodes(":rocket: Ready :white_check_mark: (:not_real:)"),
            "🚀 Ready ✅ (:not_real:)"
        );
        let plain = "No colons here";
        assert!(matches!(replace_emoji_shortcodes(plain), Cow::Borrowed(_)));
    }
}
