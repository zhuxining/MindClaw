use super::types::SlashCommandAction;

pub struct SlashCommandParser;

impl SlashCommandParser {
    pub fn parse(input: &str) -> SlashCommandAction {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return SlashCommandAction::PlainText(input.to_string());
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let command = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default().trim().to_string();

        match command {
            "/help" => SlashCommandAction::Help,
            "/default" => SlashCommandAction::ResetConversation,
            "/use" if !rest.is_empty() => SlashCommandAction::SwitchAgent { agent_id: rest },
            "/skill" if !rest.is_empty() => SlashCommandAction::SelectSkill { skill_id: rest },
            other => SlashCommandAction::Execute {
                agent_id: other.trim_start_matches('/').to_string(),
                skill_id: None,
                content: rest,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text() {
        assert_eq!(
            SlashCommandParser::parse("hello"),
            SlashCommandAction::PlainText("hello".to_string())
        );
    }

    #[test]
    fn parses_use_command() {
        assert_eq!(
            SlashCommandParser::parse("/use reviewer"),
            SlashCommandAction::SwitchAgent {
                agent_id: "reviewer".to_string()
            }
        );
    }

    #[test]
    fn parses_execute_command() {
        assert_eq!(
            SlashCommandParser::parse("/review check this"),
            SlashCommandAction::Execute {
                agent_id: "review".to_string(),
                skill_id: None,
                content: "check this".to_string()
            }
        );
    }
}
