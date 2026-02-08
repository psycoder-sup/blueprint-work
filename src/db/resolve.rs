#[derive(Debug, PartialEq)]
pub(crate) enum IdKind {
    EpicShortId,
    TaskShortId,
    Ulid,
}

pub(crate) fn classify_id(input: &str) -> IdKind {
    let upper = input.to_uppercase();
    if let Some(rest) = upper.strip_prefix('E') {
        if let Some(dash_pos) = rest.find("-T") {
            let epic_num = &rest[..dash_pos];
            let task_num = &rest[dash_pos + 2..];
            if !epic_num.is_empty()
                && epic_num.chars().all(|c| c.is_ascii_digit())
                && !task_num.is_empty()
                && task_num.chars().all(|c| c.is_ascii_digit())
            {
                return IdKind::TaskShortId;
            }
        }
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return IdKind::EpicShortId;
        }
    }
    IdKind::Ulid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_epic_short_id() {
        assert_eq!(classify_id("E1"), IdKind::EpicShortId);
        assert_eq!(classify_id("E42"), IdKind::EpicShortId);
        assert_eq!(classify_id("e1"), IdKind::EpicShortId);
        assert_eq!(classify_id("e99"), IdKind::EpicShortId);
    }

    #[test]
    fn test_classify_task_short_id() {
        assert_eq!(classify_id("E1-T3"), IdKind::TaskShortId);
        assert_eq!(classify_id("E10-T42"), IdKind::TaskShortId);
        assert_eq!(classify_id("e1-t3"), IdKind::TaskShortId);
        assert_eq!(classify_id("e2-t10"), IdKind::TaskShortId);
    }

    #[test]
    fn test_classify_ulid() {
        assert_eq!(classify_id("01ARZ3NDEKTSV4RRFFQ69G5FAV"), IdKind::Ulid);
        assert_eq!(classify_id("some-random-string"), IdKind::Ulid);
    }

    #[test]
    fn test_classify_edge_cases() {
        assert_eq!(classify_id("E"), IdKind::Ulid);
        assert_eq!(classify_id("E1-T"), IdKind::Ulid);
        assert_eq!(classify_id("E-T1"), IdKind::Ulid);
        assert_eq!(classify_id("X1"), IdKind::Ulid);
        assert_eq!(classify_id("E1-"), IdKind::Ulid);
        assert_eq!(classify_id("E1T3"), IdKind::Ulid);
    }
}
