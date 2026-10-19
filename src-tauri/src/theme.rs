//! The in-app theme editor owns one marked block inside the user's theme.css.
//! Everything outside the block is the user's own CSS and is never touched.

use std::collections::BTreeMap;

const START: &str = "/* laverna:editor:start — managed by the in-app theme editor; edits inside are overwritten */";
const END: &str = "/* laverna:editor:end */";
const START_MARK: &str = "/* laverna:editor:start";

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ThemeError {
    #[error("invalid variable name {0:?}")]
    Name(String),
    #[error("invalid value for {0}")]
    Value(String),
}

/// Locates the managed block as a byte range including both markers.
fn block_range(css: &str) -> Option<(usize, usize)> {
    let start = css.find(START_MARK)?;
    let end_rel = css[start..].find(END)?;
    Some((start, start + end_rel + END.len()))
}

/// Variables currently set by the editor.
pub fn editor_vars(css: &str) -> BTreeMap<String, String> {
    let Some((s, e)) = block_range(css) else { return BTreeMap::new() };
    css[s..e]
        .lines()
        .filter_map(|l| {
            let l = l.trim().strip_suffix(';')?;
            let (name, value) = l.split_once(':')?;
            let name = name.trim();
            name.starts_with("--").then(|| (name.to_string(), value.trim().to_string()))
        })
        .collect()
}

fn validate(name: &str, value: &str) -> Result<(), ThemeError> {
    let body = name.strip_prefix("--").ok_or_else(|| ThemeError::Name(name.into()))?;
    if body.is_empty() || !body.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ThemeError::Name(name.into()));
    }
    // Anything that could escape the declaration or the block.
    let bad = value.trim().is_empty()
        || value.contains(['{', '}', ';', '\n', '<'])
        || value.contains("/*")
        || value.contains("*/");
    if bad {
        return Err(ThemeError::Value(name.into()));
    }
    Ok(())
}

/// Rewrites the managed block with `vars`. An empty map removes the block.
/// The block goes at the end of the file so it wins over earlier rules of equal specificity.
pub fn with_editor_vars(css: &str, vars: &BTreeMap<String, String>) -> Result<String, ThemeError> {
    for (k, v) in vars {
        validate(k, v)?;
    }
    let rest = match block_range(css) {
        Some((s, e)) => {
            let after = css[e..].strip_prefix('\n').unwrap_or(&css[e..]);
            format!("{}{}", &css[..s], after)
        }
        None => css.to_string(),
    };
    if vars.is_empty() {
        return Ok(rest);
    }
    let mut out = rest.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(START);
    out.push_str("\n:root {\n");
    for (k, v) in vars {
        out.push_str(&format!("  {k}: {};\n", v.trim()));
    }
    out.push_str("}\n");
    out.push_str(END);
    out.push('\n');
    Ok(out)
}

/// Polls theme.css and emits its content on change. Polling keeps this dependency-free and
/// also catches editors that replace the file instead of writing in place.
pub async fn watch(core: std::sync::Arc<crate::core::Core>) {
    use tauri::Emitter;
    let modified = |p: &std::path::Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut last = modified(&core.paths.theme);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let now = modified(&core.paths.theme);
        if now != last {
            last = now;
            let css = std::fs::read_to_string(&core.paths.theme).unwrap_or_default();
            let _ = core.app.emit("theme", css);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn adds_block_after_user_css_and_reads_it_back() {
        let user = "body { letter-spacing: 1px; }\n";
        let out = with_editor_vars(user, &vars(&[("--accent", "#ff0066"), ("--radius", "4px")])).unwrap();
        assert!(out.starts_with(user.trim_end()));
        assert_eq!(editor_vars(&out), vars(&[("--accent", "#ff0066"), ("--radius", "4px")]));
    }

    #[test]
    fn replacing_keeps_css_on_both_sides() {
        let first = with_editor_vars("a {}\n", &vars(&[("--bg", "#000")])).unwrap();
        let edited_by_hand = format!("{first}\nb {{}}\n");
        let out = with_editor_vars(&edited_by_hand, &vars(&[("--bg", "#111")])).unwrap();
        assert!(out.contains("a {}"));
        assert!(out.contains("b {}"));
        assert_eq!(out.matches(START_MARK).count(), 1);
        assert_eq!(editor_vars(&out), vars(&[("--bg", "#111")]));
    }

    #[test]
    fn empty_map_removes_block_only() {
        let with = with_editor_vars("a {}\n", &vars(&[("--bg", "#000")])).unwrap();
        let out = with_editor_vars(&with, &BTreeMap::new()).unwrap();
        assert!(!out.contains(START_MARK));
        assert!(out.contains("a {}"));
    }

    #[test]
    fn empty_file_gets_just_the_block() {
        let out = with_editor_vars("", &vars(&[("--fg", "white")])).unwrap();
        assert!(out.starts_with(START_MARK));
    }

    #[test]
    fn values_with_commas_and_quotes_survive() {
        let font = r#""Inter", system-ui, sans-serif"#;
        let out = with_editor_vars("", &vars(&[("--font", font)])).unwrap();
        assert_eq!(editor_vars(&out)["--font"], font);
    }

    #[test]
    fn rejects_injection() {
        assert!(with_editor_vars("", &vars(&[("--bg", "red; } body { display:none")])).is_err());
        assert!(with_editor_vars("", &vars(&[("--bg", "red */")])).is_err());
        assert!(with_editor_vars("", &vars(&[("bg", "red")])).is_err());
        assert!(with_editor_vars("", &vars(&[("--b g", "red")])).is_err());
        assert!(with_editor_vars("", &vars(&[("--bg", "  ")])).is_err());
    }
}
