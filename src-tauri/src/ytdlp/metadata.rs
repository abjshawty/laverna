use crate::queue::Metadata;
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum Resolved {
    Single(Metadata),
    /// Entries as (url, metadata), in playlist order.
    Playlist(Vec<(String, Metadata)>),
}

fn thumbnail(v: &Value) -> Option<String> {
    if let Some(t) = v["thumbnail"].as_str() {
        return Some(t.to_string());
    }
    // Flat playlist entries only carry a list; the last one is the largest.
    v["thumbnails"]
        .as_array()?
        .iter()
        .rev()
        .find_map(|t| t["url"].as_str())
        .map(str::to_string)
}

fn meta(v: &Value) -> Metadata {
    Metadata {
        title: v["title"].as_str().map(str::to_string),
        thumbnail: thumbnail(v),
        duration: v["duration"].as_f64(),
        resolved: true,
    }
}

/// Interprets `yt-dlp -J --flat-playlist` output.
pub fn parse(json: &Value) -> Resolved {
    if json["_type"].as_str() == Some("playlist") {
        if let Some(entries) = json["entries"].as_array() {
            let entries: Vec<_> = entries
                .iter()
                .filter_map(|e| {
                    let url = e["url"].as_str().or(e["webpage_url"].as_str())?;
                    Some((url.to_string(), meta(e)))
                })
                .collect();
            if !entries.is_empty() {
                return Resolved::Playlist(entries);
            }
        }
    }
    Resolved::Single(meta(json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn single_video() {
        let v = json!({"_type": "video", "title": "Me at the zoo", "thumbnail": "https://i/t.jpg", "duration": 19});
        assert_eq!(
            parse(&v),
            Resolved::Single(Metadata {
                title: Some("Me at the zoo".into()),
                thumbnail: Some("https://i/t.jpg".into()),
                duration: Some(19.0),
                resolved: true,
            })
        );
    }

    #[test]
    fn flat_playlist_entries() {
        let v = json!({
            "_type": "playlist",
            "title": "List",
            "entries": [
                {"_type": "url", "url": "https://www.youtube.com/watch?v=a", "title": "A",
                 "thumbnails": [{"url": "small"}, {"url": "big"}], "duration": 60.0},
                {"_type": "url", "title": "no url, skipped"},
                {"_type": "url", "url": "https://www.youtube.com/watch?v=b", "title": "B"}
            ]
        });
        let Resolved::Playlist(e) = parse(&v) else { panic!() };
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].0, "https://www.youtube.com/watch?v=a");
        assert_eq!(e[0].1.thumbnail.as_deref(), Some("big"));
        assert_eq!(e[1].1.title.as_deref(), Some("B"));
    }

    #[test]
    fn empty_playlist_is_single() {
        let v = json!({"_type": "playlist", "title": "Empty", "entries": []});
        assert!(matches!(parse(&v), Resolved::Single(m) if m.title.as_deref() == Some("Empty")));
    }
}
