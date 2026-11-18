//! Context tags: tags that are only active at certain times of day.

/// Context tags with time windows.
const TIMED_TAGS: [&str; 2] = ["morning", "late"];

/// Event tags: context tags that are never active on their own, so their
/// lines only come when asked for explicitly (e.g. by the shell hook).
const EVENT_TAGS: [&str; 1] = ["fail"];

/// Whether `tag` is a context tag (`morning`, `late`) or an event tag (`fail`).
pub fn is_context_tag(tag: &str) -> bool {
    TIMED_TAGS.contains(&tag) || EVENT_TAGS.contains(&tag)
}

/// When the timed context tags are active, and how much they boost a line.
///
/// Windows are in minutes after midnight, start inclusive, end exclusive
/// (end may be 1440). An end before the start crosses midnight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextRules {
    pub morning: (u32, u32),
    pub late: (u32, u32),
    /// Weight multiplier for lines with an active context tag (at least 1).
    pub boost: f64,
}

impl Default for ContextRules {
    fn default() -> Self {
        ContextRules {
            morning: (5 * 60, 11 * 60),
            late: (22 * 60, 5 * 60),
            boost: 3.0,
        }
    }
}

impl ContextRules {
    /// The context at a local time of day, in minutes after midnight.
    pub fn at(&self, minutes: u32) -> Context {
        let m = minutes % (24 * 60);
        let inside = |(start, end): (u32, u32)| {
            if start < end {
                (start..end).contains(&m)
            } else {
                m >= start || m < end
            }
        };
        let active = [("morning", self.morning), ("late", self.late)]
            .into_iter()
            .filter(|(_, window)| inside(*window))
            .map(|(tag, _)| tag)
            .collect();
        Context {
            active,
            boost: self.boost,
        }
    }
}

/// The context tags active at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    active: Vec<&'static str>,
    pub(crate) boost: f64,
}

impl Default for Context {
    fn default() -> Self {
        Context::none()
    }
}

impl Context {
    /// No context tag is active.
    pub fn none() -> Self {
        Context {
            active: Vec::new(),
            boost: ContextRules::default().boost,
        }
    }

    /// The context at a local time of day with the default rules.
    pub fn at(minutes: u32) -> Self {
        ContextRules::default().at(minutes)
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
    fn custom_windows() {
        let rules = ContextRules {
            morning: (6 * 60, 7 * 60),
            late: (12 * 60, 24 * 60),
            boost: 2.0,
        };
        assert_eq!(rules.at(6 * 60 + 30).active(), ["morning"]);
        assert!(rules.at(5 * 60).active().is_empty());
        assert_eq!(rules.at(23 * 60 + 59).active(), ["late"]);
        assert!(rules.at(0).active().is_empty());
    }

    #[test]
    fn fail_is_never_active() {
        assert!(is_context_tag("fail"));
        for minutes in (0..24 * 60).step_by(15) {
            assert!(!Context::at(minutes).is_active("fail"));
        }
    }
}
