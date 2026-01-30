// Formatting utilities for Rhai

fn fill_space(width, text) {
    let len = text.chars().count();
    if len >= width {
        text
    } else {
        let padding = width - len;
        let result = text;
        let mut i = 0;
        while i < padding {
            result = result + " ";
            i = i + 1;
        }
        result
    }
}

fn pad_start(s, width, fill) {
    let len = s.chars().count();
    if len >= width {
        s
    } else {
        let padding = width - len;
        let mut result = "";
        let mut i = 0;
        while i < padding {
            result = result + fill;
            i = i + 1;
        }
        result + s
    }
}
