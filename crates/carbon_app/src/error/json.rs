/// Attempts to read a complete json object just before the error location from the provide body
/// to provided context to a deserialization error. The context is only assured
/// to be useful if the error was caused by a data mismatch not a syntax error or EOF.

pub fn read_json_context_back(ctx: &str, max_len: usize, string_can_end: bool) -> String {
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
        if c == ':' && in_str && count_string_opens == 1 && last_non_whitespace_char == Some('"') {
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

pub fn read_json_context_forward(ctx: &str, max_len: usize, string_can_end: bool) -> String {
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
        if c == ':' && in_str && count_string_opens == 1 && last_non_whitespace_char == Some('"') {
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

pub fn get_json_context(err: &serde_json::Error, body: &str, max_len: usize) -> String {
    if body.is_empty() {
        return body.to_owned();
    }

    let string_can_end = body.chars().next().map(|char| char == '[' || char == '{') != Some(true);

    // serde reports consumed bytes on a one-based line. A completed wrong-
    // type value therefore splits AFTER its last byte (no column subtraction).
    let line_offset = if err.line() <= 1 {
        0
    } else {
        body.bytes()
            .enumerate()
            .filter(|(_, byte)| *byte == b'\n')
            .nth(err.line().saturating_sub(2))
            .map(|(i, _)| i + 1)
            .unwrap_or(body.len())
    };
    let line_end = body[line_offset..]
        .find('\n')
        .map(|i| line_offset + i)
        .unwrap_or(body.len());
    let mut offset = line_offset + err.column().min(line_end - line_offset);
    while !body.is_char_boundary(offset) {
        offset -= 1;
    }
    let (ctx_before, ctx_after) = body.split_at(offset);

    let obj_before = read_json_context_back(ctx_before, max_len, string_can_end);
    let obj_after = read_json_context_forward(ctx_after, max_len, string_can_end);

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

#[cfg(test)]
mod task22_tests {
    use super::*;
    #[derive(Debug, serde::Deserialize)]
    struct Payload {
        bad: bool,
    }
    #[test]
    fn task22_multiline_unicode_marks_failing_value() {
        let body = "{\n  \"bad\": \"é\",\n  \"next\": \"innocent\"\n}";
        let err = serde_json::from_str::<Payload>(body).unwrap_err();
        assert_eq!((err.line(), err.column()), (2, 13));
        let context = get_json_context(&err, body, 400);
        assert!(context.contains("\"é\"<~~ "), "{context}");
    }
    #[test]
    fn task22_ascii_first_middle_last_crlf_and_blank_line_controls() {
        for body in [
            "{\"bad\":\"x\"}   ",
            "{\n\"bad\":\"x\",\n\"next\":1}",
            "{\n\"next\":1,\n\"bad\":\"x\"}",
            "{\r\n\r\n\"bad\":\"x\",\r\n\"next\":1}",
            "{\"pré\":\"é\",\"bad\":\"x\"}",
        ] {
            let err = serde_json::from_str::<Payload>(body).unwrap_err();
            let ctx = get_json_context(&err, body, 400);
            assert!(ctx.contains("\"x\"<~~ "), "{ctx}");
        }
        let parsed = serde_json::from_str::<Payload>("{\"bad\":true}").unwrap();
        assert!(parsed.bad);
    }
    #[test]
    fn task22_eof_and_syntax_remain_at_consumed_byte_boundary() {
        for body in [
            "{\"bad\":",
            "{\"bad\":\"unfinished",
            "[1,2",
            "{\"bad\":true,",
            "\"é",
        ] {
            let err = serde_json::from_str::<serde_json::Value>(body).unwrap_err();
            let ctx = get_json_context(&err, body, 0);
            assert!(ctx.ends_with("<~~ "), "{body:?} => {ctx}");
        }
        let body = "{\n\"bad\":@\n}";
        let err = serde_json::from_str::<Payload>(body).unwrap_err();
        assert!(get_json_context(&err, body, 400).contains("@<~~ "));
        let unicode = "{\"bad\":é}";
        let error = serde_json::from_str::<Payload>(unicode).unwrap_err();
        assert!(get_json_context(&error, unicode, 400).contains("<~~ é"));
        let err = serde_json::from_str::<Payload>("").unwrap_err();
        assert_eq!(get_json_context(&err, "", 400), "");
    }
    #[test]
    fn task22_unicode_bounds_and_unlimited_zero_contract() {
        let body = format!(
            "{{\"bad\":\"{}\",\"next\":\"{}\"}}",
            "é".repeat(1200),
            "界".repeat(1200)
        );
        let err = serde_json::from_str::<Payload>(&body).unwrap_err();
        let bounded = get_json_context(&err, &body, 400);
        let (before, after) = bounded.split_once("<~~ ").unwrap();
        assert!(before.chars().count() <= 405);
        assert!(after.chars().count() <= 404);
        assert!(get_json_context(&err, &body, 0).chars().count() > 1200);
        assert!(bounded.contains("é\"<~~ "));
    }
}
