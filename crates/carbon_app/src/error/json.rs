/// Attempts to read a complete json object just before the error location from the provide body
/// to provided context to a deserialization error. The context is only assured
/// to be useful if the error was caused by a data mismatch not a syntax error or EOF.

pub fn read_json_context_back(
    ctx: &str,
    max_len: usize,
    string_can_end: bool,
) -> String {
    let mut ctx: String = ctx.to_owned();

    let mut token_contexts: Vec<char> = vec![];
    let mut in_str = false;
    let mut found_open = false;
    let mut ctx_end = 0;
    let mut last_char: Option<char> = None;
    let mut last_non_whitespace_char: Option<char> = None;
    let mut count_string_opens = 0;
    let mut count_objects_found = 0;
    for (i, c) in ctx.char_indices().rev() {
        if c == ':'
            && in_str
            && count_string_opens == 1
            && last_non_whitespace_char == Some('"')
        {
            in_str = false;
            token_contexts.pop();
        }
        if c == '\\' && !in_str && last_char == Some('"') {
            if let Some(last_char) = last_char {
                token_contexts.push(last_char);
            }
            found_open = false;
            in_str = true;
            continue;
        }
        if c == '"' {
            if in_str && token_contexts.last() == Some(&c) {
                in_str = false;
                if string_can_end {
                    found_open = true;
                }
                token_contexts.pop();
            } else {
                in_str = true;
                count_string_opens += 1;
                token_contexts.push(c);
            }
        }
        if (c == ']' || c == '}') && !in_str {
            token_contexts.push(c);
        }
        if (c == '[' || c == '{')
            && !in_str
            && token_contexts.last() == Some(&json_matching_brace(c))
        {
            token_contexts.pop();
            found_open = true;
        }
        if (c == ',')
            && !in_str
            && (last_non_whitespace_char == Some('[')
                || last_non_whitespace_char == Some('{')
                || last_non_whitespace_char == Some('"'))
        {
            found_open = true;
        }

        last_char = Some(c);
        if !c.is_whitespace() {
            last_non_whitespace_char = Some(c);
        }
        if found_open && token_contexts.is_empty() {
            count_objects_found += 1;
        }
        if count_objects_found >= 2 {
            ctx_end = i;
            break;
        }
    }
    if ctx_end > 0 {
        ctx = ctx[ctx_end..].to_owned();
    }
    if max_len > 0 && ctx.chars().count() > max_len {
        let split_point = ctx.char_indices().rev().nth(max_len).unwrap().0;
        ctx = "... ".to_owned() + ctx.split_at(split_point).1;
    }
    ctx
}

pub fn read_json_context_forward(
    ctx: &str,
    max_len: usize,
    string_can_end: bool,
) -> String {
    let mut ctx: String = ctx.to_owned();

    let mut token_contexts: Vec<char> = vec![];
    let mut in_str = false;
    let mut found_close = false;
    let mut ctx_end = 0;
    let mut last_char: Option<char> = None;
    let mut last_non_whitespace_char: Option<char> = None;
    let mut count_string_opens = 0;
    let mut count_objects_found = 0;
    for (i, c) in ctx.char_indices() {
        if c == ':'
            && in_str
            && count_string_opens == 1
            && last_non_whitespace_char == Some('"')
        {
            in_str = false;
            token_contexts.pop();
        }
        if c == '\\' && !in_str && last_char == Some('"') {
            if let Some(last_char) = last_char {
                token_contexts.push(last_char);
            }
            found_close = false;
            in_str = true;
            continue;
        }
        if c == '"' {
            if in_str && token_contexts.last() == Some(&c) {
                in_str = false;
                if string_can_end {
                    found_close = true;
                }
                token_contexts.pop();
            } else {
                in_str = true;
                count_string_opens += 1;
                token_contexts.push(c);
            }
        }
        if (c == ']' || c == '}') && !in_str {
            token_contexts.pop();
            found_close = true;
        }
        if (c == '[' || c == '{')
            && !in_str
            && token_contexts.last() == Some(&json_matching_brace(c))
        {
            token_contexts.push(c);
        }
        if (c == ',')
            && !in_str
            && (last_non_whitespace_char == Some(']')
                || last_non_whitespace_char == Some('}')
                || last_non_whitespace_char == Some('"'))
        {
            found_close = true;
        }

        last_char = Some(c);
        if !c.is_whitespace() {
            last_non_whitespace_char = Some(c);
        }
        if found_close && token_contexts.is_empty() {
            count_objects_found += 1;
        }
        if count_objects_found >= 2 {
            ctx_end = i;
            break;
        }
    }
    if ctx_end > 0 {
        ctx = ctx[..ctx_end].to_owned();
    }
    if max_len > 0 && ctx.chars().count() > max_len {
        let split_point = ctx.char_indices().nth(max_len).unwrap().0;
        ctx = ctx.split_at(split_point).0.to_owned() + " ...";
    }
    ctx
}

pub fn get_json_context(
    err: &serde_json::Error,
    body: &str,
    max_len: usize,
) -> String {
    if body.is_empty() {
        return body.to_owned();
    }

    let string_can_end =
        body.chars().next().map(|char| char == '[' || char == '{')
            != Some(true);

    let line_offset = body
        .char_indices()
        .filter(|(_, c)| *c == '\n')
        .nth(err.line() - 1)
        .unwrap_or_default()
        .0;

    let (pre_line, ctx_line) = body.split_at(line_offset);

    let ctx = ctx_line.to_owned();
    let offset = ctx.char_indices().nth(err.column()).unwrap_or_default().0;
    let ctx_split = ctx.split_at(offset);

    let ctx_before = pre_line.to_owned() + ctx_split.0;
    let ctx_after = ctx_split.1.to_owned();

    let obj_before =
        read_json_context_back(&ctx_before, max_len, string_can_end);
    let obj_after =
        read_json_context_forward(&ctx_after, max_len, string_can_end);

    obj_before + "<~~ " + obj_after.as_str()
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
