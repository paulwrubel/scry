use anstyle::{AnsiColor, Style};
use assert_cmd::Command;
use assert_fs::TempDir;
use indoc::indoc;
use predicates::prelude::*;
use regex::Regex;
use serde_json::Value;
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
        .env("XDG_CONFIG_HOME", self.temp_dir.path().join("config"))
        .env("HOME", self.temp_dir.path())
        .current_dir(self.temp_dir.path());
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

/// Render `text` the same way the CLI does, so tests don't hardcode SGR codes.
fn styled(color: AnsiColor, text: &str) -> String {
    let style = Style::new().fg_color(Some(color.into()));
    format!("{style}{text}{style:#}")
}

/// A tag is rendered with a palette color chosen inside the binary, so an
/// integration test can't know the exact RGB. Assert only that the tag is
/// wrapped in an ANSI style, without naming any SGR codes.
fn tag_is_colored(tag: &str) -> impl predicates::prelude::Predicate<str> {
    predicate::str::is_match(format!(r"\x1b\[[0-9;]*m{tag}\x1b\[[0-9;]*m")).unwrap()
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
        .failure()
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
        .failure()
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
        .failure()
        .stderr(predicate::str::contains(
            "Task 999 not found in \"todolist\"",
        ));
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
        .failure()
        .stderr(predicate::str::contains(
            "Task 999 not found in \"todolist\"",
        ));
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
        ("hidden", "  1  Alpha"),
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
    .failure()
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
        &h.run(&["-p", "todolist", "project", "status", "reset-color", "todo"])
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
    .failure()
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
    .failure()
    .stderr(predicate::str::contains(
        "Status \"missing\" not found in \"todolist\"",
    ));
}

#[test]
fn color_never_leaves_output_plain() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["--color=never", "-p", "todolist", "add", "Alpha"])
        .success();

    h.run(&["--color=never", "-p", "todolist", "list"])
        .success()
        .stdout(predicate::str::contains("\u{1b}").not());
}

#[test]
fn status_list_renders_status_color() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&[
        "--color=always",
        "-p",
        "todolist",
        "project",
        "status",
        "set-color",
        "todo",
        "green",
    ])
    .success();

    // ANSI SGR 32 is green; the status name should be wrapped in it.
    h.run(&[
        "--color=always",
        "-p",
        "todolist",
        "project",
        "status",
        "list",
    ])
    .success()
    .stdout(predicate::str::contains(styled(AnsiColor::Green, "todo")));
}

#[test]
fn list_renders_priority_and_tag_colors() {
    let h = Harness::new();
    h.run(&["project", "create", "-t", "kanban", "kanban"])
        .success();
    h.run(&[
        "-p",
        "kanban",
        "add",
        "Alpha",
        "--priority",
        "high",
        "--tags",
        "work",
    ])
    .success();

    // `high` is p2 (yellow, SGR 33) and tags use 24-bit color. kanban enables
    // show_priority, so the priority label is rendered.
    h.run(&["--color=always", "-p", "kanban", "list"])
        .success()
        .stdout(
            predicate::str::contains(styled(AnsiColor::Yellow, "p2")).and(tag_is_colored("work")),
        );
}

#[test]
fn show_renders_status_priority_and_tag_colors() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&[
        "-p",
        "todolist",
        "add",
        "Alpha",
        "--priority",
        "high",
        "--tags",
        "work",
    ])
    .success();
    h.run(&[
        "--color=always",
        "-p",
        "todolist",
        "project",
        "status",
        "set-color",
        "todo",
        "green",
    ])
    .success();

    h.run(&["--color=always", "-p", "todolist", "show", "1"])
        .success()
        .stdout(
            predicate::str::contains(styled(AnsiColor::Green, "todo"))
                .and(predicate::str::contains(styled(
                    AnsiColor::Yellow,
                    "p2 - High",
                )))
                .and(tag_is_colored("work")),
        );
}

#[test]
fn status_move_up_and_down_reorder_statuses() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    assert_stdout(
        &h.run(&["-p", "todolist", "project", "status", "list"])
            .success(),
        indoc! {r#"
            Statuses for "todolist":
              todo
              done
        "#},
    );

    // moving the first status up is a no-op
    h.run(&["-p", "todolist", "project", "status", "move-up", "todo"])
        .success()
        .stderr(predicate::str::contains("already at the top"));

    // move-down actually reorders
    assert_stdout(
        &h.run(&["-p", "todolist", "project", "status", "move-down", "todo"])
            .success(),
        "Moved status \"todo\" down in project \"todolist\"",
    );
    assert_stdout(
        &h.run(&["-p", "todolist", "project", "status", "list"])
            .success(),
        indoc! {r#"
            Statuses for "todolist":
              done
              todo
        "#},
    );

    // moving the last status down is a no-op
    h.run(&["-p", "todolist", "project", "status", "move-down", "todo"])
        .success()
        .stderr(predicate::str::contains("already at the bottom"));

    // move-up brings it back
    assert_stdout(
        &h.run(&["-p", "todolist", "project", "status", "move-up", "todo"])
            .success(),
        "Moved status \"todo\" up in project \"todolist\"",
    );
    assert_stdout(
        &h.run(&["-p", "todolist", "project", "status", "list"])
            .success(),
        indoc! {r#"
            Statuses for "todolist":
              todo
              done
        "#},
    );
}

#[test]
fn status_move_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&["-p", "todolist", "project", "status", "move-up", "missing"])
        .failure()
        .stderr(predicate::str::contains(
            "Status \"missing\" not found in \"todolist\"",
        ));

    h.run(&[
        "-p",
        "todolist",
        "project",
        "status",
        "move-down",
        "missing",
    ])
    .failure()
    .stderr(predicate::str::contains(
        "Status \"missing\" not found in \"todolist\"",
    ));
}

#[test]
fn project_entry_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    // default entry status is `todo`
    assert_stdout(
        &h.run(&["-p", "todolist", "add", "Alpha"]).success(),
        "Created task 1 in \"todolist\" [todo]: Alpha",
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "project", "set-entry-status", "done"])
            .success(),
        "Set entry status of project \"todolist\" to \"done\"",
    );
    assert_stdout(
        &h.run(&["-p", "todolist", "add", "Beta"]).success(),
        "Created task 2 in \"todolist\" [done]: Beta",
    );

    // resetting falls back to the first status (`todo`)
    assert_stdout(
        &h.run(&["-p", "todolist", "project", "reset-entry-status"])
            .success(),
        "Reset entry status of project \"todolist\"",
    );
    assert_stdout(
        &h.run(&["-p", "todolist", "add", "Gamma"]).success(),
        "Created task 3 in \"todolist\" [todo]: Gamma",
    );
}

#[test]
fn project_set_sort_mode() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Beta"]).success();
    h.run(&["-p", "todolist", "add", "Alpha"]).success();

    // the default is manual order (insertion/position)
    assert_stdout(
        &h.run(&["-p", "todolist", "list"]).success(),
        indoc! {r#"
            project "todolist"

            * todo (2):
              1  [ ]  Beta
              2  [ ]  Alpha

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "project", "set-sort", "alphabetical"])
            .success(),
        "Set sort mode of project \"todolist\" to \"alphabetical\"",
    );

    assert_stdout(
        &h.run(&["-p", "todolist", "list"]).success(),
        indoc! {r#"
            project "todolist"

            * todo (2):
              2  [ ]  Alpha
              1  [ ]  Beta

            done (0):
        "#},
    );
}

#[test]
fn project_priority_visibility() {
    let h = Harness::new();
    h.run(&["project", "create", "-t", "kanban", "kanban"])
        .success();
    h.run(&["-p", "kanban", "add", "Alpha", "--priority", "high"])
        .success();

    // kanban shows priority by default
    assert_stdout(
        &h.run(&["-p", "kanban", "list"]).success(),
        indoc! {r#"
            project "kanban"

            in-progress (0):
            * backlog (1):
              1  p2  Alpha

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "kanban", "project", "hide-priority"])
            .success(),
        "Hiding priority in project \"kanban\"",
    );
    assert_stdout(
        &h.run(&["-p", "kanban", "list"]).success(),
        indoc! {r#"
            project "kanban"

            in-progress (0):
            * backlog (1):
              1  Alpha

            done (0):
        "#},
    );

    assert_stdout(
        &h.run(&["-p", "kanban", "project", "show-priority"])
            .success(),
        "Showing priority in project \"kanban\"",
    );
    assert_stdout(
        &h.run(&["-p", "kanban", "list"]).success(),
        indoc! {r#"
            project "kanban"

            in-progress (0):
            * backlog (1):
              1  p2  Alpha

            done (0):
        "#},
    );
}

#[test]
fn project_set_entry_status_rejects_unknown_status() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    h.run(&["-p", "todolist", "project", "set-entry-status", "missing"])
        .failure()
        .stderr(predicate::str::contains(
            "Status \"missing\" not found in \"todolist\"",
        ));
}

/// Parse JSON from a command's stdout.
fn stdout_json(assert: &assert_cmd::assert::Assert) -> Value {
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8 stdout");
    serde_json::from_str(&stdout).expect("stdout is valid JSON")
}

/// Parse the JSON error document from a command's stderr.
fn stderr_json(assert: &assert_cmd::assert::Assert) -> Value {
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8 stderr");
    serde_json::from_str(&stderr).expect("stderr is valid JSON")
}

#[test]
fn json_task_shapes_and_notes() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    let created = stdout_json(
        &h.run(&[
            "--json",
            "-p",
            "todolist",
            "add",
            "Alpha",
            "--description",
            "a desc",
            "--priority",
            "critical",
            "--tags",
            "work,urgent",
        ])
        .success(),
    );
    assert_eq!(created["id"], 1);
    assert_eq!(created["title"], "Alpha");
    assert_eq!(created["description"], "a desc");
    assert_eq!(created["priority"], "critical");
    assert_eq!(created["tags"], serde_json::json!(["urgent", "work"]));

    let listed = stdout_json(&h.run(&["--json", "-p", "todolist", "list"]).success());
    assert_eq!(listed.as_array().expect("array").len(), 1);
    assert_eq!(listed[0]["id"], 1);

    let before_notes = stdout_json(&h.run(&["--json", "-p", "todolist", "show", "1"]).success());
    assert_eq!(before_notes["notes"], serde_json::json!([]));

    stdout_json(
        &h.run(&["--json", "-p", "todolist", "note", "add", "1", "hello"])
            .success(),
    );

    let shown = stdout_json(&h.run(&["--json", "-p", "todolist", "show", "1"]).success());
    assert_eq!(shown["notes"].as_array().expect("notes").len(), 1);
    assert_eq!(shown["notes"][0]["contents"], "hello");
}

#[test]
fn json_enum_values_are_kebab_case() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    let colored = stdout_json(
        &h.run(&[
            "--json",
            "-p",
            "todolist",
            "project",
            "status",
            "set-color",
            "todo",
            "dark-gray",
        ])
        .success(),
    );
    assert_eq!(colored["color"], "dark-gray");

    let tasked = stdout_json(
        &h.run(&[
            "--json",
            "-p",
            "todolist",
            "add",
            "Beta",
            "--priority",
            "high",
        ])
        .success(),
    );
    assert_eq!(tasked["priority"], "high");

    let sorted = stdout_json(
        &h.run(&[
            "--json",
            "-p",
            "todolist",
            "project",
            "set-sort",
            "alphabetical-case-insensitive",
        ])
        .success(),
    );
    assert_eq!(sorted["task_sorting_mode"], "alphabetical-case-insensitive");
}

#[test]
fn json_suppresses_color_even_with_color_always() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();

    h.run(&["--json", "--color=always", "-p", "todolist", "list"])
        .success()
        .stdout(predicate::str::contains("\u{1b}").not());
}

#[test]
fn json_mutations_return_affected_model() {
    let h = Harness::new();
    create_todolist(&h, "todolist");
    h.run(&["-p", "todolist", "add", "Alpha"]).success();

    let deleted = stdout_json(
        &h.run(&["--json", "-p", "todolist", "delete", "1"])
            .success(),
    );
    assert_eq!(deleted["id"], 1);

    let listed = stdout_json(&h.run(&["--json", "-p", "todolist", "list"]).success());
    assert_eq!(listed, serde_json::json!([]));
}

#[test]
fn json_errors_are_structured_and_nonzero() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    let missing = h
        .run(&["--json", "-p", "todolist", "show", "999"])
        .failure();
    assert_eq!(stderr_json(&missing)["error"]["kind"], "not_found");

    let usage = h
        .run(&["--json", "-p", "todolist", "update", "1"])
        .failure();
    assert_eq!(stderr_json(&usage)["error"]["kind"], "invalid");
}

#[test]
fn json_without_subcommand_errors() {
    let h = Harness::new();
    let out = h.run(&["--json"]).failure();
    assert_eq!(stderr_json(&out)["error"]["kind"], "invalid");
}

#[test]
fn json_project_commands_return_models() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    let used = stdout_json(&h.run(&["--json", "project", "use", "todolist"]).success());
    assert_eq!(used["name"], "todolist");

    let current = stdout_json(&h.run(&["--json", "project", "current"]).success());
    assert_eq!(current["name"], "todolist");

    let created = stdout_json(&h.run(&["--json", "project", "create", "scratch"]).success());
    assert_eq!(created["name"], "scratch");

    // delete auto-confirms under --json (no stdin is provided)
    let deleted = stdout_json(&h.run(&["--json", "project", "delete", "scratch"]).success());
    assert_eq!(deleted["name"], "scratch");
}

#[test]
fn json_status_commands_return_models() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    let listed = stdout_json(
        &h.run(&["--json", "-p", "todolist", "project", "status", "list"])
            .success(),
    );
    assert_eq!(listed.as_array().expect("array").len(), 2);

    let added = stdout_json(
        &h.run(&[
            "--json", "-p", "todolist", "project", "status", "add", "blocked",
        ])
        .success(),
    );
    assert_eq!(added["name"], "blocked");

    let styled = stdout_json(
        &h.run(&[
            "--json",
            "-p",
            "todolist",
            "project",
            "status",
            "set-style",
            "blocked",
            "checked",
        ])
        .success(),
    );
    assert_eq!(styled["style"], "checked");

    let removed = stdout_json(
        &h.run(&[
            "--json", "-p", "todolist", "project", "status", "remove", "blocked",
        ])
        .success(),
    );
    assert_eq!(removed["name"], "blocked");
}

#[test]
fn json_status_move_returns_post_move_position() {
    let h = Harness::new();
    create_todolist(&h, "todolist");

    // `done` starts at position 1; moving it up should report its new position.
    let moved = stdout_json(
        &h.run(&[
            "--json", "-p", "todolist", "project", "status", "move-up", "done",
        ])
        .success(),
    );
    assert_eq!(moved["name"], "done");
    assert_eq!(moved["position"], 0);
}

#[test]
fn skill_install_writes_global_claude_code_file() {
    let h = Harness::new();
    let path = h.temp_dir.path().join(".claude/skills/scry/SKILL.md");

    assert_stdout(
        &h.run(&["skill", "install", "--harness", "claude-code"])
            .success(),
        &format!("Installed scry skill to {}", path.display()),
    );

    let content = std::fs::read_to_string(&path).expect("skill file written");
    assert!(content.starts_with("---\nname: scry\n"));
}

#[test]
fn skill_install_dir_flag_writes_to_custom_skills_root() {
    let h = Harness::new();

    assert_stdout(
        &h.run(&["skill", "install", "--dir", ".opencode/skills"])
            .success(),
        "Installed scry skill to .opencode/skills/scry/SKILL.md",
    );

    assert!(
        h.temp_dir
            .path()
            .join(".opencode/skills/scry/SKILL.md")
            .is_file()
    );
}

#[test]
fn skill_install_requires_exactly_one_target() {
    let h = Harness::new();

    h.run(&["skill", "install"]).failure();

    h.run(&["skill", "install", "--harness", "agents", "--dir", "custom"])
        .failure();
}

#[test]
fn skill_install_refuses_without_force_and_force_overwrites() {
    let h = Harness::new();
    let path = h.temp_dir.path().join(".agents/skills/scry/SKILL.md");

    h.run(&["skill", "install", "--harness", "agents"])
        .success();

    h.run(&["skill", "install", "--harness", "agents"])
        .failure()
        .stderr(predicate::str::contains("--force"));

    std::fs::write(&path, "clobbered").expect("overwrite skill file");

    h.run(&["skill", "install", "--harness", "agents", "--force"])
        .success();

    let content = std::fs::read_to_string(&path).expect("skill file written");
    assert!(content.starts_with("---\nname: scry\n"));
}

#[test]
fn skill_uninstall_removes_file_and_errors_when_absent() {
    let h = Harness::new();
    let path = h.temp_dir.path().join(".agents/skills/scry/SKILL.md");

    h.run(&["skill", "install", "--harness", "agents"])
        .success();
    assert!(path.is_file());

    assert_stdout(
        &h.run(&["skill", "uninstall", "--harness", "agents"])
            .success(),
        &format!("Uninstalled scry skill from {}", path.display()),
    );
    assert!(!path.exists());

    h.run(&["skill", "uninstall", "--harness", "agents"])
        .failure()
        .stderr(predicate::str::contains("no scry skill installed"));
}

#[test]
fn skill_print_writes_nothing_to_disk() {
    let h = Harness::new();

    let assert = h.run(&["skill", "print"]).success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8 stdout");
    assert!(stdout.starts_with("---\nname: scry\n"));

    assert!(!h.temp_dir.path().join(".claude").exists());
    assert!(!h.temp_dir.path().join(".agents").exists());
    assert!(!h.temp_dir.path().join(".opencode").exists());
}

#[test]
fn skill_uninstall_dir_flag_removes_custom_install() {
    let h = Harness::new();
    let path = h.temp_dir.path().join("custom/skills/scry/SKILL.md");

    h.run(&["skill", "install", "--dir", "custom/skills"])
        .success();
    assert!(path.is_file());

    assert_stdout(
        &h.run(&["skill", "uninstall", "--dir", "custom/skills"])
            .success(),
        "Uninstalled scry skill from custom/skills/scry/SKILL.md",
    );
    assert!(!path.exists());
}

/// `scry mcp` speaks newline-delimited JSON-RPC over stdio, so the whole
/// handshake can be driven by piping requests in and reading responses out.
#[test]
fn mcp_stdio_completes_handshake_and_lists_tools() {
    let h = Harness::new();

    // 2026-07-28 replaced `initialize` with `server/discover`, so negotiate the
    // newest version that still uses `initialize`.
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"scry-test","version":"0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        "\n",
    );

    let mut cmd = h.cmd();
    cmd.arg("mcp").write_stdin(input);
    let output = cmd.output().expect("run `scry mcp`");

    assert!(
        output.status.success(),
        "scry mcp exited with {}; stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    // Every stdout line must be a protocol frame, so stray output cannot slip in.
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .expect("utf-8 stdout")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each stdout line is a JSON-RPC message"))
        .collect();

    let initialize = responses
        .iter()
        .find(|message| message["id"].as_u64() == Some(1))
        .expect("initialize response");
    assert_eq!(
        initialize["result"]["serverInfo"]["name"].as_str(),
        Some("scry")
    );
    assert_eq!(
        initialize["result"]["serverInfo"]["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );

    let tools = responses
        .iter()
        .find(|message| message["id"].as_u64() == Some(2))
        .expect("tools/list response");
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"scry_info"), "tools were: {names:?}");
}

#[test]
fn mcp_rejects_the_json_flag() {
    let h = Harness::new();
    h.run(&["--json", "mcp"])
        .failure()
        .stderr(predicate::str::contains(
            "--json is exclusive to cli commands",
        ));
}
