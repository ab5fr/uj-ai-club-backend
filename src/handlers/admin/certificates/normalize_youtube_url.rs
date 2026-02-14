use url::Url;

pub fn normalize_youtube_url(raw_url: &str) -> String {
    let trimmed = raw_url.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    let Ok(url) = Url::parse(trimmed) else {
        return trimmed.to_string();
    };

    let host = url.host_str().unwrap_or("").to_ascii_lowercase();

    let video_id = if host == "youtu.be" || host.ends_with(".youtu.be") {
        url.path_segments()
            .and_then(|mut segments| segments.next())
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
    } else if host.ends_with("youtube.com") || host.ends_with("youtube-nocookie.com") {
        let path = url.path().trim_matches('/');

        if path == "watch" {
            url.query_pairs()
                .find_map(|(key, value)| (key == "v").then(|| value.into_owned()))
                .filter(|segment| !segment.is_empty())
        } else if let Some(id) = path.strip_prefix("embed/") {
            (!id.is_empty()).then(|| id.to_string())
        } else if let Some(id) = path.strip_prefix("shorts/") {
            (!id.is_empty()).then(|| id.to_string())
        } else {
            None
        }
    } else {
        None
    };

    match video_id {
        Some(id) => format!("https://www.youtube.com/embed/{id}"),
        None => trimmed.to_string(),
    }
}
