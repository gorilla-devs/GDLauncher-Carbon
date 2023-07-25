/// Attempts to read a complete json object just before the error location from the provide body
/// to provided context to a deserialization error. The context is only assured
/// to be useful if the error was caused by a data mismatch not a syntax error or EOF.
pub fn get_json_context(err: &serde_json::Error, body: &str, max_len: usize) -> String {
    let line_offset = body
        .char_indices()
        .filter(|(_, c)| *c == '\n')
        .nth(err.line() - 1)
        .unwrap_or_default()
        .0;
    let (pre_line, ctx_line) = body.split_at(line_offset);
    let mut ctx = ctx_line.to_owned();
    let offset = ctx.char_indices().nth(err.column()).unwrap().0;
    ctx = ctx.split_at(offset).0.to_owned();
    ctx = pre_line.to_owned() + &ctx;

    let mut token_contexts: Vec<char> = vec![];
    let mut string_open_pre = false;
    let mut in_str = false;
    let mut found_open = false;
    let mut found_close = false;
    let mut ctx_end = 0;
    let mut last_char: char = ctx.chars().rev().next().unwrap_or_default();
    for (i, c) in ctx.char_indices().rev() {
        if c == '\\' && !in_str && string_open_pre {
            token_contexts.push(last_char);
            found_open = false;
            continue;
        }
        if c == '"' || c == '\'' {
            if in_str && token_contexts.last() == Some(&c) {
                in_str = false;
                found_open = true;
                token_contexts.pop();
            } else {
                in_str = true;
                found_close = true;
                token_contexts.push(c);
            }
        }
        if (c == ']' || c == '}') && !in_str {
            token_contexts.push(c);
            found_close = true;
        }
        if (c == '[' || c == '{')
            && !in_str
            && token_contexts.last() == Some(&json_matching_brace(c))
        {
            token_contexts.pop();
            found_open = true;
        }
        if (c == ',') && !in_str && found_close {
            found_open = true;
        }

        if !in_str && string_open_pre {
            string_open_pre = false;
        }

        if in_str {
            string_open_pre = true;
        }

        last_char = c;
        if found_open && token_contexts.is_empty() {
            ctx_end = i;
            break;
        }
    }

    if ctx_end > 0 {
        ctx = ctx[ctx_end..].to_owned();
    }

    if max_len > 0 && ctx.chars().count() > max_len {
        ctx = "... ".to_owned()
            + ctx
                .split_at(ctx.char_indices().rev().nth(max_len).unwrap().0)
                .0;
    }
    ctx
}

fn json_matching_brace(c: char) -> char {
    match c {
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        other => other,
    }
}
