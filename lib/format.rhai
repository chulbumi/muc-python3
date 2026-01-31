// Formatting utilities for Rhai

fn fill_space(width, text) {
    let text_len = len(text);
    if text_len >= width {
        text
    } else {
        let padding = width - text_len;
        let result = text;
        let i = 0;
        while i < padding {
            result = result + " ";
            i = i + 1;
        }
        result
    }
}

fn pad_start(s, width, fill) {
    let s_len = len(s);
    if s_len >= width {
        s
    } else {
        let padding = width - s_len;
        let result = "";
        let i = 0;
        while i < padding {
            result = result + fill;
            i = i + 1;
        }
        result + s
    }
}
