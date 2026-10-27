use url::Url;

use super::types::AppError;

const YOUTUBE_HOSTS: &[&str] = &[
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "youtu.be",
    "www.youtu.be",
];

const APPROVED_HOSTS: &[&str] = &[
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "youtu.be",
    "www.youtu.be",
    "vimeo.com",
    "www.vimeo.com",
    "player.vimeo.com",
    "facebook.com",
    "www.facebook.com",
    "m.facebook.com",
    "web.facebook.com",
    "fb.watch",
    "www.fb.watch",
    "tiktok.com",
    "www.tiktok.com",
    "m.tiktok.com",
    "vm.tiktok.com",
    "vt.tiktok.com",
    "instagram.com",
    "www.instagram.com",
    "m.instagram.com",
    "instagr.am",
    "www.instagr.am",
];

pub fn validate_media_url(input: &str) -> Result<String, AppError> {
    let candidate = input.trim();
    let parsed = Url::parse(candidate).map_err(|_| {
        AppError::new(
            "invalid_url",
            "That does not look like a valid video link.",
            false,
        )
    })?;

    if parsed.scheme() != "https" {
        return Err(AppError::new(
            "unsupported_scheme",
            "For safety, use a secure https:// link.",
            false,
        ));
    }

    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    if !APPROVED_HOSTS.contains(&host.as_str()) {
        return Err(AppError::new(
            "unsupported_host",
            "Use a supported YouTube, Vimeo, Facebook, TikTok, or Instagram video link.",
            false,
        ));
    }
    let authority_has_port = candidate
        .split_once("://")
        .map(|(_, rest)| {
            rest.split(['/', '?', '#'])
                .next()
                .unwrap_or_default()
                .contains(':')
        })
        .unwrap_or(false);
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || authority_has_port
    {
        return Err(AppError::new(
            "unsupported_url",
            "This link format is not supported. Paste a standard video link.",
            false,
        ));
    }
    if parsed.query_pairs().any(|(key, _)| key == "list") {
        return Err(AppError::new(
            "playlist_not_supported",
            "Playlist links are not supported. Open an individual video and copy its link.",
            false,
        ));
    }

    if YOUTUBE_HOSTS.contains(&host.as_str()) {
        let video_id = youtube_video_id(&parsed).ok_or_else(|| {
            AppError::new(
                "invalid_video_url",
                "We could not find an individual video in that link.",
                false,
            )
        })?;

        if video_id.len() != 11
            || !video_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(AppError::new(
                "invalid_video_url",
                "We could not find an individual video in that link.",
                false,
            ));
        }
        return Ok(format!("https://www.youtube.com/watch?v={video_id}"));
    }

    if !is_individual_platform_video(&parsed, &host) {
        return Err(AppError::new(
            "invalid_video_url",
            "Paste a link to an individual video, not a profile or collection.",
            false,
        ));
    }

    let mut normalized = parsed;
    normalized.set_fragment(None);
    Ok(normalized.to_string())
}

fn is_individual_platform_video(url: &Url, host: &str) -> bool {
    let path = url.path();
    let clean_path = path.trim_end_matches('/');
    if matches!(host, "vimeo.com" | "www.vimeo.com") {
        let id = path
            .strip_prefix("/video/")
            .or_else(|| path.strip_prefix('/'))
            .unwrap_or_default()
            .trim_end_matches('/');
        return !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit());
    }
    if host == "player.vimeo.com" {
        let id = path
            .strip_prefix("/video/")
            .unwrap_or_default()
            .trim_end_matches('/');
        return !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit());
    }
    if matches!(
        host,
        "facebook.com" | "www.facebook.com" | "m.facebook.com" | "web.facebook.com"
    ) {
        if clean_path == "/watch" {
            return url
                .query_pairs()
                .any(|(key, value)| key == "v" && value.bytes().all(|byte| byte.is_ascii_digit()));
        }
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        return matches!(segments.as_slice(), ["reel", id] if valid_slug(id))
            || matches!(segments.as_slice(), ["videos", id] if valid_slug(id))
            || matches!(segments.as_slice(), ["videos", _, id] if valid_slug(id))
            || matches!(segments.as_slice(), ["share", "v", id] if valid_slug(id))
            || matches!(segments.as_slice(), ["share", "r", id] if valid_slug(id));
    }
    if matches!(
        host,
        "fb.watch" | "www.fb.watch" | "vm.tiktok.com" | "vt.tiktok.com"
    ) {
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        return matches!(segments.as_slice(), [id] if valid_slug(id));
    }
    if matches!(host, "tiktok.com" | "www.tiktok.com" | "m.tiktok.com") {
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        return matches!(segments.as_slice(), [user, "video", id] if user.starts_with('@') && valid_slug(id))
            || matches!(segments.as_slice(), ["t", id] if valid_slug(id));
    }
    if matches!(
        host,
        "instagram.com" | "www.instagram.com" | "m.instagram.com" | "instagr.am" | "www.instagr.am"
    ) {
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        return matches!(segments.as_slice(), [kind, id] if matches!(*kind, "p" | "reel" | "reels" | "tv") && valid_slug(id));
    }
    false
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:".contains(&byte))
}

fn youtube_video_id(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    if host == "youtu.be" || host == "www.youtu.be" {
        let mut parts = url.path_segments()?;
        let id = parts.next()?.to_string();
        return if parts.next().is_none() && !id.is_empty() {
            Some(id)
        } else {
            None
        };
    }

    let mut segments = url.path_segments()?;
    let first = segments.next()?;
    match first {
        "watch" if url.path() == "/watch" => url
            .query_pairs()
            .find_map(|(key, value)| (key == "v").then(|| value.into_owned())),
        "shorts" | "embed" | "live" => {
            let id = segments.next()?.to_string();
            if segments.next().is_none() && !id.is_empty() {
                Some(id)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_media_url;

    #[test]
    fn accepts_and_normalizes_supported_individual_video_urls() {
        for url in [
            "https://www.youtube.com/watch?v=BaW_jenozKc",
            "https://youtu.be/BaW_jenozKc?t=4",
            "https://m.youtube.com/shorts/BaW_jenozKc",
            "https://www.youtube.com/embed/BaW_jenozKc",
            "https://www.youtube.com/live/BaW_jenozKc",
        ] {
            assert_eq!(
                validate_media_url(url).unwrap(),
                "https://www.youtube.com/watch?v=BaW_jenozKc"
            );
        }
    }

    #[test]
    fn rejects_untrusted_schemes_hosts_credentials_ports_and_playlists() {
        for url in [
            "http://youtube.com/watch?v=BaW_jenozKc",
            "file:///C:/video.mp4",
            "https://youtube.com.attacker.example/watch?v=BaW_jenozKc",
            "https://attacker-youtube.com/watch?v=BaW_jenozKc",
            "https://user@youtube.com/watch?v=BaW_jenozKc",
            "https://youtube.com:443/watch?v=BaW_jenozKc",
            "https://youtube.com/watch?v=BaW_jenozKc&list=PL123",
            "https://youtube.com/watch?v=short",
        ] {
            assert!(validate_media_url(url).is_err(), "{url}");
        }
    }
}
