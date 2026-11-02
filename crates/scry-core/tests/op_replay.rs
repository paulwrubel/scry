mod common;

use common::{FixtureCase, load_area};
use scry_core::models::Change;
use scry_core::sync::operations::Operation;
use scry_core::sync::replay::{ClientState, apply_changes, replay_operations, visible_state};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayInput {
    confirmed: ClientState,
    #[serde(default)]
    ops: Vec<Operation>,
    #[serde(default)]
    changes: Vec<Change>,
    #[serde(default)]
    acked: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayExpected {
    #[serde(default)]
    visible: Option<ClientState>,
    #[serde(default)]
    confirmed: Option<ClientState>,
}

#[test]
fn op_replay_matches_the_shared_fixtures() {
    let file = load_area("op-replay");

    for case in &file.cases {
        let input: ReplayInput = deserialize(case);
        let expected: ReplayExpected = serde_json::from_value(case.expected.clone())
            .unwrap_or_else(|error| panic!("case `{}` has invalid expected: {error}", case.name));

        match case.kind.as_str() {
            "replay" => {
                let actual = replay_operations(&input.confirmed, &input.ops);
                assert_eq!(actual, state(case, expected.visible, "visible"));
            }
            "apply_changes" => {
                let actual = apply_changes(&input.confirmed, &input.changes);
                assert_eq!(actual, state(case, expected.confirmed, "confirmed"));
            }
            "cycle" => {
                let actual =
                    visible_state(&input.confirmed, &input.changes, &input.acked, &input.ops);
                assert_eq!(actual, state(case, expected.visible, "visible"));
            }
            other => panic!("case `{}` uses unknown fixture kind `{other}`", case.name),
        }
    }
}

fn state(case: &FixtureCase, state: Option<ClientState>, field: &str) -> ClientState {
    state.unwrap_or_else(|| panic!("case `{}` is missing `{field}`", case.name))
}

fn deserialize<T>(case: &FixtureCase) -> T
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(case.input.clone())
        .unwrap_or_else(|error| panic!("case `{}` has an invalid input: {error}", case.name))
}
