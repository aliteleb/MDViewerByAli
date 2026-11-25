/// Offline embedded KaTeX v0.16.22 assets (JS, CSS, and 20 WOFF2 fonts).
pub fn lookup_katex_asset(path: &str) -> Option<(&'static [u8], &'static str)> {
    match path {
        "/vendor/katex/katex.min.js" => Some((
            include_bytes!("../assets/vendor/katex/katex.min.js"),
            "application/javascript; charset=utf-8",
        )),
        "/vendor/katex/katex.min.css" => Some((
            include_bytes!("../assets/vendor/katex/katex.min.css"),
            "text/css; charset=utf-8",
        )),
        "/vendor/katex/fonts/KaTeX_AMS-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_AMS-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Caligraphic-Bold.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Caligraphic-Bold.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Caligraphic-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Caligraphic-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Fraktur-Bold.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Fraktur-Bold.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Fraktur-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Fraktur-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Main-Bold.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Main-Bold.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Main-BoldItalic.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Main-BoldItalic.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Main-Italic.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Main-Italic.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Main-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Main-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Math-BoldItalic.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Math-BoldItalic.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Math-Italic.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Math-Italic.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_SansSerif-Bold.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_SansSerif-Bold.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_SansSerif-Italic.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_SansSerif-Italic.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_SansSerif-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_SansSerif-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Script-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Script-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Size1-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Size1-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Size2-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Size2-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Size3-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Size3-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Size4-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Size4-Regular.woff2"),
            "font/woff2",
        )),
        "/vendor/katex/fonts/KaTeX_Typewriter-Regular.woff2" => Some((
            include_bytes!("../assets/vendor/katex/fonts/KaTeX_Typewriter-Regular.woff2"),
            "font/woff2",
        )),
        _ => None,
    }
}
