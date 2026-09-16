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
    create_todolist(&h, "todolist");

    assert_stdout(
        &h.run(&[
            "-p",
            "todolist",
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
        "Created task 1 in \"todolist\" [todo]: Alpha",
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "show", "1"]).success(),
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
    h.run(&["project", "create", "blank"]).success();
    h.run(&["-p", "blank", "project", "status", "add", "todo"])
        .success();

    assert_stdout(
        &h.run(&["-p", "blank", "add", "Alpha"]).success(),
        "Created task 1 in \"blank\" [todo]: Alpha",
    );
}

#[test]
fn add_accepts_explicit_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    assert_stdout(
        &h.run(&["-p", "todolist", "add", "Alpha", "--status", "done"])
            .success(),
        "Created task 1 in \"todolist\" [done]: Alpha",
    );
}

#[test]
fn add_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&["-p", "todolist", "add", "Alpha", "--status", "missing"])
        .success()
        .stderr(predicate::str::contains(
            "Status \"missing\" not found in \"todolist\"",
        ));
}

#[test]
fn update_changes_fields_and_clears_description_and_tags() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&[
        "-p",
        "todolist",
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
            "todolist",
            "update",
            "1",
            "--title",
            "Delta",
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
        &h.run(&["-p", "todolist", "show", "1"]).success(),
        indoc! {r#"
            Delta #1


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
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();

    h.run(&["-p", "todolist", "update", "1"])
        .success()
        .stderr(predicate::str::contains("No flags provided"));
}

#[test]
fn duplicate_copies_the_source_position() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();
    h.run(&["-p", "todolist", "add", "Beta"]).success();

    assert_stdout(
        &h.run(&["-p", "todolist", "duplicate", "1"]).success(),
        "Duplicated task 1 as task 3 in \"todolist\" [todo]",
    );

    // Manual sort keeps the copied position, so the duplicate (id 3) sits next
    // to the source (id 1, position 0), ahead of Beta (id 2, position 1).
    assert_stdout(
        &h.run(&["-p", "todolist", "list"]).success(),
        indoc! {r#"
            project "todolist"

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
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();
    h.run(&["-p", "todolist", "add", "Beta", "--status", "done"])
        .success();

    assert_stdout(
        &h.run(&["-p", "todolist", "list"]).success(),
        indoc! {r#"
            project "todolist"

            * todo (1):
              1  [ ]  Alpha

            done (1):
              2  [x]  Beta
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "list", "--status", "done"])
            .success(),
        indoc! {r#"
            project "todolist"

            done (1):
              2  [x]  Beta
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "list", "--status", "missing"])
            .success(),
        indoc! {r#"
            project "todolist"

            No tasks.
        "#},
    );
}

#[test]
fn list_search_matches_titles_and_tags() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha", "--tags", "work"])
        .success();
    h.run(&["-p", "todolist", "add", "Beta", "--tags", "home"])
        .success();
    h.run(&["-p", "todolist", "add", "Gamma", "--tags", "exercise"])
        .success();

    assert_stdout(
        &h.run(&["-p", "todolist", "list", "--search", "work"])
            .success(),
        indoc! {r#"
            project "todolist"

            * todo (1):
              1  [ ]  Alpha  work

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "list", "--search", "EXERC"])
            .success(),
        indoc! {r#"
            project "todolist"

            * todo (1):
              3  [ ]  Gamma  exercise

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "list", "--search", "nomatch"])
            .success(),
        indoc! {r#"
            project "todolist"

            No tasks.
        "#},
    );
}

#[test]
fn list_respects_show_priority_and_project_sort_mode() {
    let h = Harness::new();
    h.run(&["project", "create", "-t", "kanban", "kanban"])
        .success();
    h.run(&["-p", "kanban", "add", "Alpha", "--priority", "low"])
        .success();
    h.run(&["-p", "kanban", "add", "Beta", "--priority", "critical"])
        .success();

    // kanban enables show_priority and sorts by Priority. `Priority`'s ordering
    // is by discriminant, so the most urgent (Beta, p1) sorts before Alpha (p4).
    assert_stdout(
        &h.run(&["-p", "kanban", "list"]).success(),
        indoc! {r#"
            project "kanban"

            in-progress (0):
            * backlog (2):
              2  p1  Beta
              1  p4  Alpha

            done (0):
        "#},
    );
}

#[test]
fn show_missing_task_reports_to_stderr() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&["-p", "todolist", "show", "999"])
        .success()
        .stderr(predicate::str::contains("Task 999 not found in \"todolist\""));
}

#[test]
fn note_add_appears_in_show() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();

    assert_stdout(
        &h.run(&[
            "-p",
            "todolist",
            "note",
            "add",
            "1",
            "first note line\nsecond note line",
        ])
        .success(),
        "Added note 1 to task 1.",
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "show", "1"]).success(),
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
    create_todolist(&h, "todolist");

    h.run(&["-p", "todolist", "note", "add", "999", "a note"])
        .success()
        .stderr(predicate::str::contains("Task 999 not found in \"todolist\""));
}

#[test]
fn status_set_style() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();

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
                "todolist",
                "project",
                "status",
                "set-style",
                "todo",
                style,
            ])
            .success(),
            &format!("Set style of status \"todo\" to \"{style}\" in project \"todolist\""),
        );

        assert_stdout(
            &h.run(&["-p", "todolist", "list"]).success(),
            &format!("project \"todolist\"\n\n* todo (1):\n{task_line}\n\ndone (0):"),
        );
    }
}

#[test]
fn status_set_style_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&[
        "-p",
        "todolist",
        "project",
        "status",
        "set-style",
        "missing",
        "checked",
    ])
    .success()
    .stderr(predicate::str::contains(
        "Status \"missing\" not found in \"todolist\"",
    ));
}

#[test]
fn status_color() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    // Color isn't rendered in any CLI output, so this only exercises the
    // accept path and confirmations.
    assert_stdout(
        &h.run(&[
            "-p",
            "todolist",
            "project",
            "status",
            "set-color",
            "todo",
            "green",
        ])
        .success(),
        "Set color of status \"todo\" to \"green\" in project \"todolist\"",
    );

    assert_stdout(
        &h.run(&[
            "-p",
            "todolist",
            "project",
            "status",
            "reset-color",
            "todo",
        ])
        .success(),
        "Reset color of status \"todo\" in project \"todolist\"",
    );
}

#[test]
fn status_color_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&[
        "-p",
        "todolist",
        "project",
        "status",
        "set-color",
        "missing",
        "green",
    ])
    .success()
    .stderr(predicate::str::contains(
        "Status \"missing\" not found in \"todolist\"",
    ));

    h.run(&[
        "-p",
        "todolist",
        "project",
        "status",
        "reset-color",
        "missing",
    ])
    .success()
    .stderr(predicate::str::contains(
        "Status \"missing\" not found in \"todolist\"",
    ));
}
