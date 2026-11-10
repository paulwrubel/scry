//! `automation` commands.

use scry_core::models::{
    Automation, AutomationAction, AutomationTrigger, Priority, Project, Section,
};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// The trigger chosen on the command line.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum AutomationTriggerKind {
    Created,
    Closed,
    Opened,
    Enters,
}

/// The action chosen on the command line.
#[derive(Clone, Debug)]
pub enum AutomationActionChoice {
    MoveTo(String),
    NoSection,
    Close,
    Open,
    Priority(Priority),
}

/// List a project's automations.
pub async fn automation_list(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let automations = fetch_automations(client, &project).await?;

    if output_json {
        print_json(&automations)?;
        return Ok(());
    }

    if automations.is_empty() {
        println!("No automations configured for \"{}\".", project.name);
        return Ok(());
    }

    let sections = fetch_sections(client, &project).await?;
    println!("Automations for \"{}\":", project.name);
    for automation in &automations {
        println!("  {}", describe_automation(automation, &sections));
    }

    Ok(())
}

/// Create or update the action for one trigger.
pub async fn automation_set(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    when: AutomationTriggerKind,
    trigger_section: Option<&str>,
    action: AutomationActionChoice,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let trigger = resolve_trigger(client, &project, when, trigger_section).await?;
    let action = resolve_action(client, &project, action).await?;
    let sections = fetch_sections(client, &project).await?;
    let automations = fetch_automations(client, &project).await?;
    let existing = automations
        .iter()
        .find(|automation| automation.trigger == trigger);

    #[derive(Serialize)]
    struct CreateAutomationRequest<'a> {
        trigger: &'a AutomationTrigger,
        actions: &'a [AutomationAction],
    }

    #[derive(Serialize)]
    struct UpdateAutomationRequest<'a> {
        actions: &'a [AutomationAction],
    }

    let actions = [action];
    let project_id = project.id.to_string();
    let (automation, created) = match existing {
        Some(existing) => {
            let automation_id = existing.id.to_string();
            let automation: Automation = client
                .put_json(
                    &["automations", automation_id.as_str()],
                    &[],
                    &UpdateAutomationRequest { actions: &actions },
                )
                .await?;
            (automation, false)
        }
        None => {
            let automation: Automation = client
                .post_json(
                    &["projects", project_id.as_str(), "automations"],
                    &[],
                    &CreateAutomationRequest {
                        trigger: &trigger,
                        actions: &actions,
                    },
                )
                .await?;
            (automation, true)
        }
    };

    if output_json {
        print_json(&automation)?;
    } else if created {
        println!(
            "Created automation: {}",
            describe_automation(&automation, &sections)
        );
    } else {
        println!(
            "Updated automation: {}",
            describe_automation(&automation, &sections)
        );
    }

    Ok(())
}

/// Remove the automation for one trigger.
pub async fn automation_remove(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    when: AutomationTriggerKind,
    trigger_section: Option<&str>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let trigger = resolve_trigger(client, &project, when, trigger_section).await?;
    let sections = fetch_sections(client, &project).await?;
    let automations = fetch_automations(client, &project).await?;
    let Some(existing) = automations
        .iter()
        .find(|automation| automation.trigger == trigger)
    else {
        return Err(AppError::Usage(format!(
            "no automation for {}",
            trigger_label(&trigger, &sections)
        )));
    };

    let automation_id = existing.id.to_string();
    let removed: Automation = client
        .delete_json(&["automations", automation_id.as_str()], &[])
        .await?;

    if output_json {
        print_json(&removed)?;
    } else {
        println!(
            "Removed automation: {}",
            describe_automation(&removed, &sections)
        );
    }

    Ok(())
}

async fn resolve_trigger(
    client: &Client,
    project: &Project,
    when: AutomationTriggerKind,
    section: Option<&str>,
) -> Result<AutomationTrigger, AppError> {
    match (when, section) {
        (AutomationTriggerKind::Created, None) => Ok(AutomationTrigger::TaskCreated),
        (AutomationTriggerKind::Closed, None) => Ok(AutomationTrigger::TaskClosed),
        (AutomationTriggerKind::Opened, None) => Ok(AutomationTrigger::TaskOpened),
        (AutomationTriggerKind::Enters, Some(name)) => {
            let section = section_by_name(client, project, name).await?;
            Ok(AutomationTrigger::TaskEntersSection {
                section_id: section.id,
            })
        }
        (AutomationTriggerKind::Enters, None) => Err(AppError::Usage(
            "--section is required with --when enters".to_string(),
        )),
        (_, Some(_)) => Err(AppError::Usage(
            "--section is only valid with --when enters".to_string(),
        )),
    }
}

async fn resolve_action(
    client: &Client,
    project: &Project,
    choice: AutomationActionChoice,
) -> Result<AutomationAction, AppError> {
    match choice {
        AutomationActionChoice::MoveTo(name) => {
            let section = section_by_name(client, project, &name).await?;
            Ok(AutomationAction::MoveToSection {
                section_id: Some(section.id),
            })
        }
        AutomationActionChoice::NoSection => {
            Ok(AutomationAction::MoveToSection { section_id: None })
        }
        AutomationActionChoice::Close => Ok(AutomationAction::CloseTask),
        AutomationActionChoice::Open => Ok(AutomationAction::OpenTask),
        AutomationActionChoice::Priority(priority) => {
            Ok(AutomationAction::SetPriority { priority })
        }
    }
}

async fn fetch_automations(
    client: &Client,
    project: &Project,
) -> Result<Vec<Automation>, AppError> {
    let project_id = project.id.to_string();
    client
        .get_json(&["projects", project_id.as_str(), "automations"], &[])
        .await
}

async fn fetch_sections(client: &Client, project: &Project) -> Result<Vec<Section>, AppError> {
    let project_id = project.id.to_string();
    client
        .get_json(&["projects", project_id.as_str(), "sections"], &[])
        .await
}

fn trigger_label(trigger: &AutomationTrigger, sections: &[Section]) -> String {
    match trigger {
        AutomationTrigger::TaskCreated => "When a task is created".to_string(),
        AutomationTrigger::TaskClosed => "When a task is closed".to_string(),
        AutomationTrigger::TaskOpened => "When a task is opened".to_string(),
        AutomationTrigger::TaskEntersSection { section_id } => {
            let name = sections
                .iter()
                .find(|section| section.id == *section_id)
                .map(|section| section.name.as_str())
                .unwrap_or("a section");
            format!("When a task enters {name}")
        }
    }
}

fn action_label(action: &AutomationAction, sections: &[Section]) -> String {
    match action {
        AutomationAction::MoveToSection {
            section_id: Some(section_id),
        } => {
            let name = sections
                .iter()
                .find(|section| section.id == *section_id)
                .map(|section| section.name.as_str())
                .unwrap_or("a section");
            format!("move to {name}")
        }
        AutomationAction::MoveToSection { section_id: None } => "move to No section".to_string(),
        AutomationAction::CloseTask => "close it".to_string(),
        AutomationAction::OpenTask => "open it".to_string(),
        AutomationAction::SetPriority { priority } => format!("set priority to {priority}"),
    }
}

fn describe_automation(automation: &Automation, sections: &[Section]) -> String {
    let trigger = trigger_label(&automation.trigger, sections);
    let actions = automation
        .actions
        .iter()
        .map(|action| action_label(action, sections))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{trigger} -> {actions}")
}
