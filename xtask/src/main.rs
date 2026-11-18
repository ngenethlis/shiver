//! Repository tasks. Run with `cargo xtask <task>`.
//!
//! - `check-specs`: check every spec folder in `docs/specs/` follows the
//!   format described in `AGENTS.md`.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const STATUSES: [&str; 4] = ["draft", "approved", "in-progress", "done"];
const STYLESHEET_LINK: &str = r#"href="../../../../style.css""#;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("check-specs") => {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask lives inside the workspace");
            match check_specs(&root.join("docs/specs")) {
                Ok(report) => {
                    for p in &report.problems {
                        let path = p.path.strip_prefix(root).unwrap_or(&p.path);
                        println!("{}: {}", path.display(), p.message);
                    }
                    if report.problems.is_empty() {
                        println!("specs ok ({} specs)", report.specs);
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("xtask: can't read docs/specs: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("usage: cargo xtask check-specs");
            ExitCode::from(2)
        }
    }
}

/// The result of checking all specs.
#[derive(Debug, Default)]
struct Report {
    specs: usize,
    problems: Vec<Problem>,
}

#[derive(Debug)]
struct Problem {
    path: PathBuf,
    message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl Report {
    fn problem(&mut self, path: &Path, message: impl Into<String>) {
        self.problems.push(Problem {
            path: path.to_path_buf(),
            message: message.into(),
        });
    }
}

/// Checks every spec under `specs` (`docs/specs`). A missing folder has no specs.
fn check_specs(specs: &Path) -> io::Result<Report> {
    let mut report = Report::default();
    let mut ids: BTreeMap<String, PathBuf> = BTreeMap::new();

    for year in sorted_entries(specs)? {
        let year_name = file_name(&year);
        if !year.is_dir() || !is_digits(&year_name, 4) {
            report.problem(&year, "expected a year folder (yyyy)");
            continue;
        }
        for month in sorted_entries(&year)? {
            let month_name = file_name(&month);
            let valid_month = is_digits(&month_name, 2)
                && (1..=12).contains(&month_name.parse::<u32>().unwrap_or(0));
            if !month.is_dir() || !valid_month {
                report.problem(&month, "expected a month folder (01-12)");
                continue;
            }
            for spec in sorted_entries(&month)? {
                let Some((id, name)) = split_spec_folder(&file_name(&spec)) else {
                    report.problem(
                        &spec,
                        "expected a spec folder named <4-digit id>-<kebab-case-name>",
                    );
                    continue;
                };
                if !spec.is_dir() {
                    report.problem(&spec, "expected a spec folder, found a file");
                    continue;
                }
                if let Some(first) = ids.get(&id) {
                    let msg = format!("id {id} is already used by {}", first.display());
                    report.problem(&spec, msg);
                } else {
                    ids.insert(id.clone(), spec.clone());
                }
                report.specs += 1;
                let folder = Folder {
                    year: &year_name,
                    month: &month_name,
                    id: &id,
                    name: &name,
                };
                check_spec(&spec, &folder, &mut report)?;
            }
        }
    }
    Ok(report)
}

/// What a spec folder's path says about the spec.
struct Folder<'a> {
    year: &'a str,
    month: &'a str,
    id: &'a str,
    name: &'a str,
}

fn check_spec(dir: &Path, folder: &Folder, report: &mut Report) -> io::Result<()> {
    let spec_path = dir.join("spec.md");
    let onepager_path = dir.join("onepager.html");
    let tasks_path = dir.join("tasks.md");

    if !onepager_path.is_file() {
        report.problem(dir, "missing onepager.html");
    }
    if !spec_path.is_file() {
        report.problem(dir, "missing spec.md");
        return Ok(());
    }
    let spec = fs::read_to_string(&spec_path)?;

    let Some(front) = front_matter(&spec) else {
        report.problem(&spec_path, "missing front matter (--- block at the top)");
        return Ok(());
    };
    for key in ["id", "name", "status", "created"] {
        if !front.contains_key(key) {
            report.problem(&spec_path, format!("front matter is missing `{key}`"));
        }
    }
    if let Some(id) = front.get("id")
        && id != folder.id
    {
        let msg = format!("id `{id}` doesn't match folder id `{}`", folder.id);
        report.problem(&spec_path, msg);
    }
    if let Some(name) = front.get("name")
        && name != folder.name
    {
        let msg = format!("name `{name}` doesn't match folder name `{}`", folder.name);
        report.problem(&spec_path, msg);
    }
    if let Some(created) = front.get("created") {
        match parse_date(created) {
            None => report.problem(
                &spec_path,
                format!("created `{created}` isn't a valid yyyy-mm-dd date"),
            ),
            Some((y, m)) if y != folder.year || m != folder.month => report.problem(
                &spec_path,
                format!(
                    "created `{created}` doesn't match folder {}/{}",
                    folder.year, folder.month
                ),
            ),
            Some(_) => {}
        }
    }

    let status = front.get("status").map(String::as_str);
    if let Some(s) = status
        && !STATUSES.contains(&s)
    {
        let msg = format!("status `{s}` isn't one of {}", STATUSES.join(", "));
        report.problem(&spec_path, msg);
    }

    for requirement in requirements_without_acceptance(&spec) {
        let msg = format!("requirement `{requirement}` has no acceptance criteria");
        report.problem(&spec_path, msg);
    }

    if onepager_path.is_file() {
        let onepager = fs::read_to_string(&onepager_path)?;
        if !onepager.contains(STYLESHEET_LINK) {
            report.problem(
                &onepager_path,
                format!("doesn't link the stylesheet ({STYLESHEET_LINK})"),
            );
        }
        match (onepager_status(&onepager), status) {
            (None, _) => report.problem(&onepager_path, r#"missing <span class="status">…</span>"#),
            (Some(shown), Some(s)) if shown != s => report.problem(
                &onepager_path,
                format!("shows status `{shown}` but spec.md says `{s}`"),
            ),
            _ => {}
        }
    }

    let started = matches!(status, Some("in-progress" | "done"));
    if started && !tasks_path.is_file() {
        report.problem(dir, "missing tasks.md (required once work has started)");
    }

    if status == Some("done") {
        for (path, text) in [
            (&spec_path, Some(spec.clone())),
            (&tasks_path, read_if_exists(&tasks_path)?),
        ] {
            if let Some(text) = text {
                let open = outside_code(&text).filter(|l| is_open_box(l)).count();
                if open > 0 {
                    report.problem(
                        path,
                        format!("status is done but {open} box(es) are unticked"),
                    );
                }
            }
        }
    }
    Ok(())
}

/// Parses `---\nkey: value\n---` at the top of a file.
fn front_matter(text: &str) -> Option<BTreeMap<String, String>> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut map = BTreeMap::new();
    for line in lines {
        if line.trim_end() == "---" {
            return Some(map);
        }
        if let Some((key, value)) = line.split_once(':') {
            map.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    None
}

/// Requirement headings (`### R<n>: ...`) whose section lacks an
/// `**Acceptance:**` line followed by at least one checkbox.
fn requirements_without_acceptance(spec: &str) -> Vec<String> {
    let mut missing = Vec::new();
    let mut current: Option<(String, bool, bool)> = None; // (heading, acceptance, checkbox)
    let mut finish = |current: &mut Option<(String, bool, bool)>| {
        if let Some((heading, acceptance, checkbox)) = current.take()
            && !(acceptance && checkbox)
        {
            missing.push(heading);
        }
    };
    for line in outside_code(spec) {
        if line.starts_with("## ") || line.starts_with("### ") {
            finish(&mut current);
            if let Some(heading) = line.strip_prefix("### ")
                && is_requirement_heading(heading)
            {
                current = Some((heading.to_owned(), false, false));
            }
        } else if let Some((_, acceptance, checkbox)) = &mut current {
            if line.trim() == "**Acceptance:**" {
                *acceptance = true;
            } else if *acceptance && is_box(line) {
                *checkbox = true;
            }
        }
    }
    finish(&mut current);
    missing
}

/// `R12: Something`.
fn is_requirement_heading(heading: &str) -> bool {
    heading
        .strip_prefix('R')
        .and_then(|rest| rest.split_once(':'))
        .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// The text inside `<span class="status">…</span>`.
fn onepager_status(html: &str) -> Option<&str> {
    let start = html.find(r#"<span class="status">"#)? + r#"<span class="status">"#.len();
    let len = html[start..].find("</span>")?;
    Some(html[start..start + len].trim())
}

/// Lines that aren't inside ``` fenced code blocks.
fn outside_code(text: &str) -> impl Iterator<Item = &str> {
    let mut in_code = false;
    text.lines().filter(move |line| {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            return false;
        }
        !in_code
    })
}

fn is_box(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("- [ ]") || l.starts_with("- [x]")
}

fn is_open_box(line: &str) -> bool {
    line.trim_start().starts_with("- [ ]")
}

/// `0001-some-name` → (`0001`, `some-name`).
fn split_spec_folder(folder: &str) -> Option<(String, String)> {
    let (id, name) = folder.split_once('-')?;
    let kebab = !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    (is_digits(id, 4) && kebab).then(|| (id.to_owned(), name.to_owned()))
}

/// Validates `yyyy-mm-dd`, returning the year and month strings.
fn parse_date(date: &str) -> Option<(&str, &str)> {
    let mut parts = date.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || !is_digits(y, 4) || !is_digits(m, 2) || !is_digits(d, 2) {
        return None;
    }
    let (year, month, day): (u32, u32, u32) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (1..=days).contains(&day).then_some((y, m))
}

fn is_digits(s: &str, len: usize) -> bool {
    s.len() == len && s.chars().all(|c| c.is_ascii_digit())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Entries of `dir`, sorted; a missing `dir` has none.
fn sorted_entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut paths = entries
        .map(|e| e.map(|e| e.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}

fn read_if_exists(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary `docs/specs` folder.
    struct Specs(tempfile::TempDir);

    impl Specs {
        fn new() -> Self {
            Specs(tempfile::tempdir().unwrap())
        }

        fn root(&self) -> &Path {
            self.0.path()
        }

        /// Writes a valid spec with `status` at `<rel>` and returns its folder.
        fn add(&self, rel: &str, status: &str) -> PathBuf {
            let dir = self.root().join(rel);
            let (id, name) = split_spec_folder(&file_name(&dir)).unwrap();
            let parts: Vec<_> = rel.split('/').collect();
            let created = format!("{}-{}-05", parts[0], parts[1]);
            let tick = if status == "done" { "x" } else { " " };
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("spec.md"),
                format!(
                    "---\nid: {id}\nname: {name}\nstatus: {status}\ncreated: {created}\n---\n\n\
                     # {id}: Test\n\n## Requirements\n\n\
                     ### R1: Something\n\nText.\n\n**Acceptance:**\n- [{tick}] Test `a` passes.\n\n\
                     ## Out of scope\n\n- Nothing.\n"
                ),
            )
            .unwrap();
            fs::write(dir.join("onepager.html"), onepager(status)).unwrap();
            if status == "in-progress" || status == "done" {
                fs::write(
                    dir.join("tasks.md"),
                    format!("# Tasks\n\n- [{tick}] 1. Do it (R1)\n"),
                )
                .unwrap();
            }
            dir
        }

        fn check(&self) -> Report {
            check_specs(self.root()).unwrap()
        }
    }

    fn onepager(status: &str) -> String {
        format!(
            "<link rel=\"stylesheet\" href=\"../../../../style.css\">\n\
             <p class=\"meta\"><span class=\"status\">{status}</span></p>\n"
        )
    }

    fn edit(path: &Path, from: &str, to: &str) {
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains(from), "{from:?} not in {}", path.display());
        fs::write(path, text.replacen(from, to, 1)).unwrap();
    }

    /// Asserts exactly one problem, mentioning `text`.
    fn assert_one(report: &Report, text: &str) {
        assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
        let p = report.problems[0].to_string();
        assert!(p.contains(text), "`{p}` should mention `{text}`");
    }

    #[test]
    fn accepts_valid_specs() {
        let specs = Specs::new();
        specs.add("2026/11/0001-first", "draft");
        specs.add("2026/11/0002-second-one", "approved");
        specs.add("2026/12/0003-third", "in-progress");
        specs.add("2027/01/0004-fourth", "done");
        let report = specs.check();
        assert!(report.problems.is_empty(), "{:#?}", report.problems);
        assert_eq!(report.specs, 4);
    }

    #[test]
    fn missing_specs_folder_is_ok() {
        let report = check_specs(Path::new("/nonexistent/docs/specs")).unwrap();
        assert_eq!(report.specs, 0);
        assert!(report.problems.is_empty());
    }

    #[test]
    fn rejects_bad_year_or_month() {
        let specs = Specs::new();
        specs.add("26/11/0001-a", "draft");
        assert_one(&specs.check(), "expected a year folder");

        let specs = Specs::new();
        specs.add("2026/13/0001-a", "draft");
        assert_one(&specs.check(), "expected a month folder");

        let specs = Specs::new();
        fs::write(specs.root().join("README.md"), "").unwrap();
        assert_one(&specs.check(), "expected a year folder");
    }

    #[test]
    fn rejects_bad_folder_name() {
        for bad in [
            "1-a",
            "0001",
            "0001-Name",
            "0001-a_b",
            "0001--a",
            "0001-a-",
            "abcd-a",
        ] {
            let specs = Specs::new();
            fs::create_dir_all(specs.root().join("2026/11").join(bad)).unwrap();
            assert_one(&specs.check(), "expected a spec folder named");
        }
    }

    #[test]
    fn rejects_duplicate_ids() {
        let specs = Specs::new();
        specs.add("2026/11/0001-a", "draft");
        specs.add("2026/12/0001-b", "draft");
        assert_one(&specs.check(), "id 0001 is already used");
    }

    #[test]
    fn requires_spec_and_onepager() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        fs::remove_file(dir.join("onepager.html")).unwrap();
        assert_one(&specs.check(), "missing onepager.html");

        fs::write(dir.join("onepager.html"), onepager("draft")).unwrap();
        fs::remove_file(dir.join("spec.md")).unwrap();
        assert_one(&specs.check(), "missing spec.md");
    }

    #[test]
    fn requires_tasks_once_started() {
        for status in ["in-progress", "done"] {
            let specs = Specs::new();
            let dir = specs.add("2026/11/0001-a", status);
            fs::remove_file(dir.join("tasks.md")).unwrap();
            assert_one(&specs.check(), "missing tasks.md");
        }
    }

    #[test]
    fn requires_front_matter_fields() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(&dir.join("spec.md"), "created: 2026-11-05\n", "");
        assert_one(&specs.check(), "missing `created`");

        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(&dir.join("spec.md"), "---\nid", "id");
        assert_one(&specs.check(), "missing front matter");
    }

    #[test]
    fn front_matter_matches_folder() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(&dir.join("spec.md"), "id: 0001", "id: 0002");
        assert_one(&specs.check(), "doesn't match folder id");

        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(&dir.join("spec.md"), "name: a", "name: b");
        assert_one(&specs.check(), "doesn't match folder name");

        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(
            &dir.join("spec.md"),
            "created: 2026-11-05",
            "created: 2026-10-05",
        );
        assert_one(&specs.check(), "doesn't match folder 2026/11");
    }

    #[test]
    fn rejects_unknown_status() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(&dir.join("spec.md"), "status: draft", "status: wip");
        edit(&dir.join("onepager.html"), ">draft<", ">wip<");
        assert_one(&specs.check(), "status `wip` isn't one of");
    }

    #[test]
    fn rejects_bad_created_date() {
        for bad in [
            "2026-11-31",
            "2026-11-5",
            "2026/11/05",
            "2026-02-29",
            "soon",
        ] {
            let specs = Specs::new();
            let dir = specs.add("2026/11/0001-a", "draft");
            edit(
                &dir.join("spec.md"),
                "created: 2026-11-05",
                &format!("created: {bad}"),
            );
            assert_one(&specs.check(), "isn't a valid yyyy-mm-dd date");
        }
        assert_eq!(parse_date("2028-02-29"), Some(("2028", "02")), "leap year");
    }

    #[test]
    fn requirement_needs_acceptance() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(
            &dir.join("spec.md"),
            "**Acceptance:**\n- [ ] Test `a` passes.\n",
            "",
        );
        assert_one(
            &specs.check(),
            "requirement `R1: Something` has no acceptance",
        );

        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(&dir.join("spec.md"), "- [ ] Test `a` passes.\n", "");
        assert_one(&specs.check(), "has no acceptance");
    }

    #[test]
    fn ignores_headings_in_code_blocks() {
        let spec = "## Requirements\n\n```\n### R9: In a code block\n```\n\n\
                    ### R1: Real\n\n**Acceptance:**\n- [ ] ok\n";
        assert!(requirements_without_acceptance(spec).is_empty());
    }

    #[test]
    fn onepager_links_stylesheet() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        edit(
            &dir.join("onepager.html"),
            "../../../../style.css",
            "../style.css",
        );
        assert_one(&specs.check(), "doesn't link the stylesheet");
    }

    #[test]
    fn onepager_status_matches_spec() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "approved");
        edit(&dir.join("onepager.html"), ">approved<", ">draft<");
        assert_one(
            &specs.check(),
            "shows status `draft` but spec.md says `approved`",
        );

        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "draft");
        fs::write(dir.join("onepager.html"), r#"href="../../../../style.css""#).unwrap();
        assert_one(&specs.check(), "missing <span class=\"status\">");
    }

    #[test]
    fn done_spec_has_no_open_boxes() {
        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "done");
        edit(&dir.join("tasks.md"), "- [x] 1.", "- [ ] 1.");
        assert_one(
            &specs.check(),
            "tasks.md: status is done but 1 box(es) are unticked",
        );

        let specs = Specs::new();
        let dir = specs.add("2026/11/0001-a", "done");
        edit(&dir.join("spec.md"), "- [x] Test", "- [ ] Test");
        assert_one(
            &specs.check(),
            "spec.md: status is done but 1 box(es) are unticked",
        );
    }

    #[test]
    fn reports_all_problems_not_just_first() {
        let specs = Specs::new();
        let a = specs.add("2026/11/0001-a", "draft");
        let b = specs.add("2026/11/0002-b", "draft");
        edit(&a.join("spec.md"), "name: a", "name: x");
        edit(&b.join("spec.md"), "status: draft", "status: nope");
        fs::remove_file(b.join("onepager.html")).unwrap();
        let report = specs.check();
        assert_eq!(report.problems.len(), 3, "{:#?}", report.problems);
    }
}
