//! Crew brief rendering from a versioned template.
//!
//! Rendering is pure: the template text is supplied by the caller (the `bh`
//! binary embeds `prompts/crew-brief.md`), and only `{{key}}` placeholders are
//! substituted. It is testable without a filesystem or a clock.

/// Values substituted into a brief template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefContext {
    /// Repo id.
    pub repo: String,
    /// Task id.
    pub task_id: String,
    /// Task title.
    pub title: String,
    /// Absolute worktree path.
    pub worktree: String,
    /// Branch the crewmate works on.
    pub branch: String,
    /// Delivery mode name.
    pub delivery: String,
    /// Absolute status-log path.
    pub status_log: String,
    /// Boundary note for the dispatch policy.
    pub boundaries: String,
    /// The task body from the backlog item.
    pub body: String,
}

impl BriefContext {
    fn values(&self) -> [(&str, &str); 9] {
        [
            ("repo", &self.repo),
            ("task_id", &self.task_id),
            ("title", &self.title),
            ("worktree", &self.worktree),
            ("branch", &self.branch),
            ("delivery", &self.delivery),
            ("status_log", &self.status_log),
            ("boundaries", &self.boundaries),
            ("body", &self.body),
        ]
    }
}

/// Replaces every `{{key}}` placeholder with its value.
///
/// Unknown placeholders are left untouched, so a newer template is obvious in
/// the rendered brief rather than silently blank.
pub fn render(template: &str, context: &BriefContext) -> String {
    let mut rendered = template.to_owned();
    for (key, value) in context.values() {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> BriefContext {
        BriefContext {
            repo: "bulkhead".to_owned(),
            task_id: "t-1".to_owned(),
            title: "Add a classifier".to_owned(),
            worktree: "/tmp/wt".to_owned(),
            branch: "bh/t-1".to_owned(),
            delivery: "no-mistakes".to_owned(),
            status_log: "/tmp/wt/status.log".to_owned(),
            boundaries: "bh-core".to_owned(),
            body: "Do the thing.".to_owned(),
        }
    }

    #[test]
    fn substitutes_every_placeholder() {
        let rendered = render(
            "{{repo}}/{{task_id}} {{title}} -> {{worktree}} {{branch}} {{delivery}} {{status_log}} {{boundaries}} {{body}}",
            &context(),
        );
        assert_eq!(
            rendered,
            "bulkhead/t-1 Add a classifier -> /tmp/wt bh/t-1 no-mistakes /tmp/wt/status.log bh-core Do the thing."
        );
    }

    #[test]
    fn leaves_unknown_placeholders() {
        assert_eq!(render("{{unknown}}", &context()), "{{unknown}}");
    }
}
