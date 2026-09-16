use assert_cmd::Command;
use assert_fs::TempDir;
use indoc::indoc;
use predicates::prelude::*;
use regex::Regex;
use std::sync::LazyLock;

/// Matches the `%b %-d, %Y at %-I:%M%P` timestamps rendered by `scry show`.
static TIMESTAMP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Z][a-z]{2} \d{1,2}, \d{4} at \d{1,2}:\d{2}(am|pm)")
        .expect("valid timestamp regex")
});

/// Isolated CLI harness: each instance gets its own scratch database and config
/// directory so tests never touch real user data and can run in parallel.
struct Harness {
    temp_dir: TempDir,
}

impl Harness {
    fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("create temp dir"),
        }
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("scry").expect("scry binary is built");
        cmd.env(
            "DATABASE_URL",
            format!(
                "sqlite://{}",
                self.temp_dir.path().join("scry.db").display()
            ),
        )
        .env("XDG_CONFIG_HOME", self.temp_dir.path().join("config"));
        cmd
    }

    fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cmd().args(args).assert()
    }
}

/// Run a command, assert success, and compare its full stdout to `expected`,
/// after scrubbing values that legitimately vary between runs. Keeping the whole
/// expected output as a literal catches formatting regressions that a handful of
/// `contains` checks would miss.
fn assert_stdout(assert: &assert_cmd::assert::Assert, expected: &str) {
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8 stdout");
    assert_eq!(normalize(&stdout), expected.trim_end());
}

/// Replaces volatile timestamps with a placeholder and ignores line-trailing
/// whitespace, so output can be compared against a stable literal.
fn normalize(stdout: &str) -> String {
    stdout
        .lines()
        .map(|line| {
            TIMESTAMP_REGEX
                .replace_all(line.trim_end(), "[TIMESTAMP]")
                .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn create_todolist(h: &Harness, name: &str) {
    h.run(&["project", "create", "-t", "todolist", name])
        .success();
}

#[test]
fn add_persists_description_priority_tags_and_entry_status() {
    let h = Harness::new();
    create_todolist(&h, "smoke");

    assert_stdout(
        &h.run(&[
            "-p",
            "smoke",
            "add",
            "Alpha",
            "--description",
            "a description",
            "--priority",
            "high",
            "--tags",
            "work, urgent",
        ])
        .success(),
        "Created task 1 in \"smoke\" [todo]: Alpha",
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "show", "1"]).success(),
        indoc! {r#"
            Alpha #1

                a description

            Priority:    p2 - High
            Status:      todo
            Tags:        urgent work
            Created at:  [TIMESTAMP]
        "#},
    );
}

#[test]
fn add_defaults_to_first_status_without_entry_status() {
    let h = Harness::new();
    h.run(&["project", "create", "plain"]).success();
    h.run(&["-p", "plain", "project", "status", "add", "only"])
        .success();

    assert_stdout(
        &h.run(&["-p", "plain", "add", "Solo"]).success(),
        "Created task 1 in \"plain\" [only]: Solo",
    );
}

#[test]
fn add_accepts_explicit_status() {
    let h = Harness::new();
    create_todolist(&h, "smoke");

    assert_stdout(
        &h.run(&["-p", "smoke", "add", "Done item", "--status", "done"])
            .success(),
        "Created task 1 in \"smoke\" [done]: Done item",
    );
}

#[test]
fn add_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "smoke");

    h.run(&["-p", "smoke", "add", "Nope", "--status", "missing"])
        .success()
        .stderr(predicate::str::contains(
            "Status \"missing\" not found in \"smoke\"",
        ));
}

#[test]
fn update_changes_fields_and_clears_description_and_tags() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&[
        "-p",
        "smoke",
        "add",
        "Alpha",
        "--description",
        "a description",
        "--priority",
        "low",
        "--tags",
        "work",
    ])
    .success();

    assert_stdout(
        &h.run(&[
            "-p",
            "smoke",
            "update",
            "1",
            "--title",
            "Renamed",
            "--description",
            "",
            "--tags",
            "",
            "--priority",
            "p1",
        ])
        .success(),
        "Updated task 1.",
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "show", "1"]).success(),
        indoc! {r#"
            Renamed #1


            Priority:    p1 - Critical
            Status:      todo
            Tags:
            Created at:  [TIMESTAMP]
        "#},
    );
}

#[test]
fn update_requires_at_least_one_flag() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&["-p", "smoke", "add", "Alpha"]).success();

    h.run(&["-p", "smoke", "update", "1"])
        .success()
        .stderr(predicate::str::contains("No flags provided"));
}

#[test]
fn duplicate_copies_the_source_position() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&["-p", "smoke", "add", "Alpha"]).success();
    h.run(&["-p", "smoke", "add", "Beta"]).success();

    assert_stdout(
        &h.run(&["-p", "smoke", "duplicate", "1"]).success(),
        "Duplicated task 1 as task 3 in \"smoke\" [todo]",
    );

    // Manual sort keeps the copied position, so the duplicate (id 3) sits next
    // to the source (id 1, position 0), ahead of Beta (id 2, position 1).
    assert_stdout(
        &h.run(&["-p", "smoke", "list"]).success(),
        indoc! {r#"
            project "smoke"

            * todo (3):
              1  [ ]  Alpha
              3  [ ]  Alpha
              2  [ ]  Beta

            done (0):
        "#},
    );
}

#[test]
fn list_marks_the_entry_status_and_filters_by_status() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&["-p", "smoke", "add", "Todo item"]).success();
    h.run(&["-p", "smoke", "add", "Done item", "--status", "done"])
        .success();

    assert_stdout(
        &h.run(&["-p", "smoke", "list"]).success(),
        indoc! {r#"
            project "smoke"

            * todo (1):
              1  [ ]  Todo item

            done (1):
              2  [x]  Done item
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "list", "--status", "done"])
            .success(),
        indoc! {r#"
            project "smoke"

            done (1):
              2  [x]  Done item
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "list", "--status", "missing"])
            .success(),
        indoc! {r#"
            project "smoke"

            No tasks.
        "#},
    );
}

#[test]
fn list_search_matches_titles_and_tags() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&["-p", "smoke", "add", "Alpha", "--tags", "work"])
        .success();
    h.run(&["-p", "smoke", "add", "Beta", "--tags", "home"])
        .success();
    h.run(&["-p", "smoke", "add", "Gamma exercise"]).success();

    assert_stdout(
        &h.run(&["-p", "smoke", "list", "--search", "work"])
            .success(),
        indoc! {r#"
            project "smoke"

            * todo (1):
              1  [ ]  Alpha  work

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "list", "--search", "EXERC"])
            .success(),
        indoc! {r#"
            project "smoke"

            * todo (1):
              3  [ ]  Gamma exercise

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "list", "--search", "nomatch"])
            .success(),
        indoc! {r#"
            project "smoke"

            No tasks.
        "#},
    );
}

#[test]
fn list_respects_show_priority_and_project_sort_mode() {
    let h = Harness::new();
    h.run(&["project", "create", "-t", "kanban", "kb"])
        .success();
    h.run(&["-p", "kb", "add", "Low", "--priority", "low"])
        .success();
    h.run(&["-p", "kb", "add", "Crit", "--priority", "critical"])
        .success();

    // kanban enables show_priority and sorts by Priority. `Priority`'s ordering
    // is by discriminant, so the most urgent (Crit, p1) sorts before Low (p4).
    assert_stdout(
        &h.run(&["-p", "kb", "list"]).success(),
        indoc! {r#"
            project "kb"

            in-progress (0):
            * backlog (2):
              2  p1  Crit
              1  p4  Low

            done (0):
        "#},
    );
}

#[test]
fn show_missing_task_reports_to_stderr() {
    let h = Harness::new();
    create_todolist(&h, "smoke");

    h.run(&["-p", "smoke", "show", "999"])
        .success()
        .stderr(predicate::str::contains("Task 999 not found in \"smoke\""));
}

#[test]
fn note_add_appears_in_show() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&["-p", "smoke", "add", "Alpha"]).success();

    assert_stdout(
        &h.run(&[
            "-p",
            "smoke",
            "note",
            "add",
            "1",
            "first note line\nsecond note line",
        ])
        .success(),
        "Added note 1 to task 1.",
    );

    assert_stdout(
        &h.run(&["-p", "smoke", "show", "1"]).success(),
        indoc! {r#"
            Alpha #1


            Priority:    p3 - Medium
            Status:      todo
            Tags:
            Created at:  [TIMESTAMP]

            [TIMESTAMP]
            first note line
            second note line
        "#},
    );
}

#[test]
fn note_add_rejects_unknown_task() {
    let h = Harness::new();
    create_todolist(&h, "smoke");

    h.run(&["-p", "smoke", "note", "add", "999", "orphan"])
        .success()
        .stderr(predicate::str::contains("Task 999 not found in \"smoke\""));
}

#[test]
fn status_set_style() {
    let h = Harness::new();
    create_todolist(&h, "smoke");
    h.run(&["-p", "smoke", "add", "Alpha"]).success();

    // (style token, the expected task line in `scry list`)
    let cases = [
        ("none", "  1  Alpha"),
        ("unchecked", "  1  [ ]  Alpha"),
        ("checked", "  1  [x]  Alpha"),
        ("strikethrough", "  1  Alpha"),
    ];

    for (style, task_line) in cases {
        assert_stdout(
            &h.run(&[
                "-p",
                "smoke",
                "project",
                "status",
                "set-style",
                "todo",
                style,
            ])
            .success(),
            &format!("Set style of status \"todo\" to \"{style}\" in project \"smoke\""),
        );

        assert_stdout(
            &h.run(&["-p", "smoke", "list"]).success(),
            &format!("project \"smoke\"\n\n* todo (1):\n{task_line}\n\ndone (0):"),
        );
    }
}

#[test]
fn status_set_style_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "smoke");

    h.run(&[
        "-p",
        "smoke",
        "project",
        "status",
        "set-style",
        "missing",
        "checked",
    ])
    .success()
    .stderr(predicate::str::contains(
        "Status \"missing\" not found in \"smoke\"",
    ));
}
