pub fn take(t: &String, pat: &str) -> Option <String> {
    let t = t.trim();
    let pat = pat.trim();

    if pat.starts_with(t) { return Some(String::new()) }

    let mut iter = t.chars();
    let mut idx= 0;

    for (i, c) in pat.chars().enumerate() {
        if c == '.' {
            if idx == 1 {
                idx = i - 1;
                break
            }
            idx = 1;
        } else {
            idx = 0;
            if c != iter.next()? { return None }
        }
    }

    Some((&t[idx..t.rfind(&pat[idx + 2..idx + 3])?]).to_string())
}
