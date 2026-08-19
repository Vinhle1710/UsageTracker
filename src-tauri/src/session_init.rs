use std::process::{Command, Stdio};
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
    Command::new(&spec.program)
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
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
}
