#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TagCompletion {
    pub(super) tag: String,
    pub(super) typed_chars: usize,
}

pub(super) fn parse_tags(value: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for part in value.split(';') {
        let tag = part.trim();
        if tag.is_empty()
            || tags
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            continue;
        }
        tags.push(tag.to_string());
    }
    tags
}

pub(super) fn canonicalize_description(value: &str, known_tags: &[String]) -> String {
    let tags = parse_tags(value)
        .into_iter()
        .map(|tag| {
            known_tags
                .iter()
                .find(|known| known.eq_ignore_ascii_case(&tag))
                .cloned()
                .unwrap_or(tag)
        })
        .collect::<Vec<_>>();
    tags.join("; ")
}

pub(super) fn contains_tag(value: &str, tag: &str) -> bool {
    let tag = tag.trim();
    !tag.is_empty()
        && parse_tags(value)
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(tag))
}

pub(super) fn contains_all_tags(value: &str, filter: &str) -> bool {
    let requested = parse_tags(filter);
    !requested.is_empty() && requested.iter().all(|tag| contains_tag(value, tag))
}

pub(super) fn tag_completion(value: &str, known_tags: &[String]) -> Option<TagCompletion> {
    let token = value.rsplit(';').next().unwrap_or(value).trim();
    let used = if let Some(separator) = value.rfind(';') {
        parse_tags(&value[..separator])
    } else {
        Vec::new()
    };
    let token_lower = token.to_lowercase();
    known_tags
        .iter()
        .find(|known| {
            let known_lower = known.to_lowercase();
            !used
                .iter()
                .any(|used_tag| used_tag.eq_ignore_ascii_case(known))
                && known_lower.starts_with(&token_lower)
                && !known.eq_ignore_ascii_case(token)
        })
        .map(|known| TagCompletion {
            tag: known.clone(),
            typed_chars: token.chars().count(),
        })
}

pub(super) fn accept_tag_completion(value: &str, completion: &TagCompletion) -> String {
    let prefix = value
        .rfind(';')
        .map(|separator| parse_tags(&value[..separator]))
        .unwrap_or_default();
    let mut tags = prefix;
    tags.push(completion.tag.clone());
    tags.join("; ")
}

pub(super) fn replace_current_tag(value: &str, replacement: &str) -> String {
    let mut tags = if let Some(separator) = value.rfind(';') {
        parse_tags(&value[..separator])
    } else {
        Vec::new()
    };
    let replacement = replacement.trim();
    if !replacement.is_empty()
        && !tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(replacement))
    {
        tags.push(replacement.to_string());
    }
    tags.join("; ")
}

#[cfg(test)]
mod tests {
    use super::{
        accept_tag_completion, canonicalize_description, contains_all_tags, contains_tag,
        parse_tags, replace_current_tag, tag_completion,
    };

    #[test]
    fn semicolon_tags_trim_dedupe_and_preserve_first_order() {
        assert_eq!(
            parse_tags(" Renzo ; Anibal; renzo ; ; Personal "),
            vec!["Renzo", "Anibal", "Personal"]
        );
    }

    #[test]
    fn canonicalization_reuses_known_spelling() {
        let known = vec!["Renzo".to_string(), "Anibal".to_string()];
        assert_eq!(
            canonicalize_description("renzo; ANIBAL", &known),
            "Renzo; Anibal"
        );
    }

    #[test]
    fn tag_membership_is_independent_inside_multi_tag_description() {
        assert!(contains_tag("Renzo; Anibal", "renzo"));
        assert!(contains_tag("Renzo; Anibal", "Anibal"));
        assert!(!contains_tag("Renzo; Anibal", "Personal"));
        assert!(contains_all_tags("Renzo; Anibal", "Renzo; Anibal"));
        assert!(!contains_all_tags("Renzo", "Renzo; Anibal"));
    }

    #[test]
    fn completion_targets_only_the_current_semicolon_token() {
        let known = vec!["Renzo".to_string(), "Anibal".to_string()];
        let completion = tag_completion("Renzo; Ani", &known).unwrap();
        assert_eq!(completion.tag, "Anibal");
        assert_eq!(completion.typed_chars, 3);
        assert_eq!(
            accept_tag_completion("Renzo; Ani", &completion),
            "Renzo; Anibal"
        );
        assert_eq!(replace_current_tag("Renzo; A", "Anibal"), "Renzo; Anibal");
    }
}
