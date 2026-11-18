//! Context tags: tags that are only active at certain times of day.

/// Context tags and their windows, in minutes after midnight (start
/// inclusive, end exclusive). An end before the start crosses midnight.
const WINDOWS: [(&str, u32, u32); 2] = [("morning", 5 * 60, 11 * 60), ("late", 22 * 60, 5 * 60)];

/// Event tags: context tags that are never active on their own, so their
/// lines only come when asked for explicitly (e.g. by the shell hook).
const EVENT_TAGS: [&str; 1] = ["fail"];

/// How much more likely a line with an active context tag is.
pub(crate) const BOOST: f64 = 3.0;

/// Whether `tag` is a context tag (`morning`, `late`) or an event tag (`fail`).
pub fn is_context_tag(tag: &str) -> bool {
    WINDOWS.iter().any(|(t, _, _)| *t == tag) || EVENT_TAGS.contains(&tag)
}

/// The context tags active at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    active: Vec<&'static str>,
}

impl Context {
    /// No context tag is active.
    pub fn none() -> Self {
        Context::default()
    }

    /// The context at a local time of day, in minutes after midnight.
    pub fn at(minutes: u32) -> Self {
        let m = minutes % (24 * 60);
        let active = WINDOWS
            .iter()
            .filter(|(_, start, end)| {
                if start < end {
                    (*start..*end).contains(&m)
                } else {
                    m >= *start || m < *end
                }
            })
            .map(|(tag, _, _)| *tag)
            .collect();
        Context { active }
    }

    /// Whether `tag` is active.
    pub fn is_active(&self, tag: &str) -> bool {
        self.active.contains(&tag)
    }

    /// The active context tags.
    pub fn active(&self) -> &[&'static str] {
        &self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(h: u32, m: u32) -> Vec<&'static str> {
        Context::at(h * 60 + m).active().to_vec()
    }

    #[test]
    fn windows() {
        assert_eq!(at(4, 59), ["late"]);
        assert_eq!(at(5, 0), ["morning"]);
        assert_eq!(at(10, 59), ["morning"]);
        assert!(at(11, 0).is_empty());
        assert!(at(21, 59).is_empty());
        assert_eq!(at(22, 0), ["late"]);
        assert_eq!(at(0, 0), ["late"]);
        assert!(Context::none().active().is_empty());
    }

    #[test]
    fn knows_context_tags() {
        assert!(is_context_tag("late"));
        assert!(is_context_tag("morning"));
        assert!(!is_context_tag("focus"));
    }

    #[test]
    fn fail_is_never_active() {
        assert!(is_context_tag("fail"));
        for minutes in (0..24 * 60).step_by(15) {
            assert!(!Context::at(minutes).is_active("fail"));
        }
    }
}
