
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_hyphen = false;

    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_was_hyphen = false;
        } else if !last_was_hyphen && !out.is_empty() {
            out.push('-');
            last_was_hyphen = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "article".to_string()
    } else {
        out
    }
}

pub fn to_admin_article(a: crate::models::Article) -> crate::models::AdminArticleResponse {
    crate::models::AdminArticleResponse {
        id: a.id,
        title: a.title,
        slug: a.slug,
        excerpt: a.excerpt,
        body: a.body,
        cover_image: a.cover_image,
        visible: a.visible,
        created_at: a.created_at,
        updated_at: a.updated_at,
    }
}
