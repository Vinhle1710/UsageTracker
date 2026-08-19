use std::process::{Command, Stdio};
use std::time::Duration;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    Light,
    Standard,
    Reasoning,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelChoice {
    pub id: &'static str,
    pub capability: Capability,
    pub relative_cost: u16,
}
pub const MODELS: &[ModelChoice] = &[
    ModelChoice {
        id: "gpt-5.6-terra",
        capability: Capability::Standard,
        relative_cost: 10,
    },
    ModelChoice {
        id: "gpt-5.6-sol",
        capability: Capability::Reasoning,
        relative_cost: 20,
    },
];
pub fn choose_model(required: Capability, models: &[ModelChoice]) -> Option<&ModelChoice> {
    models
        .iter()
        .filter(|m| m.capability >= required)
        .min_by_key(|m| m.relative_cost)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}
pub fn session_command(model: &str) -> CommandSpec {
    CommandSpec {
        program: "codex".into(),
        args: vec![
            "exec".into(),
            "--model".into(),
            model.into(),
            "--".into(),
            "Initialize a usage-tracking session and wait.".into(),
        ],
    }
}
pub fn spawn_session(spec: &CommandSpec) -> std::io::Result<std::process::Child> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn()
}

pub const AUTO_INIT_COOLDOWN: Duration = Duration::from_secs(30 * 60);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitDecision {
    Skipped,
    Cooldown,
    Started,
    Failed,
}
pub struct InitContext {
    pub enabled: bool,
    pub acknowledged: bool,
    pub provider_active: bool,
    pub credentials_available: bool,
    pub child_or_session_live: bool,
    pub now: i64,
    pub last_attempt: Option<i64>,
}

pub fn maybe_initialize(
    context: &InitContext,
    model: &ModelChoice,
    spawn: impl FnOnce(&CommandSpec) -> bool,
) -> (InitDecision, Option<i64>) {
    if !context.enabled
        || !context.acknowledged
        || !context.provider_active
        || !context.credentials_available
        || context.child_or_session_live
    {
        return (InitDecision::Skipped, context.last_attempt);
    }
    if context
        .last_attempt
        .is_some_and(|last| context.now.saturating_sub(last) < AUTO_INIT_COOLDOWN.as_secs() as i64)
    {
        return (InitDecision::Cooldown, context.last_attempt);
    }
    let spec = session_command(model.id);
    let timestamp = Some(context.now);
    if spawn(&spec) {
        (InitDecision::Started, timestamp)
    } else {
        (InitDecision::Failed, timestamp)
    }
}

pub fn has_live_initialization_process(process_names: &[String]) -> bool {
    process_names.iter().any(|name| {
        let name = name.to_ascii_lowercase();
        name.contains("usage-tracker-auto-init") || name == "codex exec" || name == "claude exec"
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chooses_cheapest_suitable_enabled_model() {
        assert_eq!(
            choose_model(Capability::Standard, MODELS).unwrap().id,
            "gpt-5.6-terra"
        );
    }
    #[test]
    fn no_model_means_no_process() {
        assert_eq!(choose_model(Capability::Reasoning, &[]), None);
    }
    #[test]
    fn argv_never_uses_a_shell() {
        assert_eq!(
            session_command("gpt-5.6-terra"),
            CommandSpec {
                program: "codex".into(),
                args: vec![
                    "exec".into(),
                    "--model".into(),
                    "gpt-5.6-terra".into(),
                    "--".into(),
                    "Initialize a usage-tracking session and wait.".into()
                ]
            }
        );
    }
    #[test]
    fn auto_init_requires_ack_and_records_failures_for_cooldown() {
        let c = InitContext {
            enabled: true,
            acknowledged: false,
            provider_active: true,
            credentials_available: true,
            child_or_session_live: false,
            now: 1000,
            last_attempt: None,
        };
        assert_eq!(
            maybe_initialize(&c, &MODELS[0], |_| true),
            (InitDecision::Skipped, None)
        );
        let c = InitContext {
            acknowledged: true,
            ..c
        };
        assert_eq!(
            maybe_initialize(&c, &MODELS[0], |_| false),
            (InitDecision::Failed, Some(1000))
        );
        let c = InitContext {
            last_attempt: Some(1000),
            now: 1001,
            ..c
        };
        assert_eq!(
            maybe_initialize(&c, &MODELS[0], |_| true),
            (InitDecision::Cooldown, Some(1000))
        );
    }
    #[test]
    fn live_initialization_process_blocks_a_second_child() {
        assert!(has_live_initialization_process(&[
            "usage-tracker-auto-init".into()
        ]));
        assert!(!has_live_initialization_process(&["ChatGPT.exe".into()]));
    }
}
