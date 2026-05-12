use std::{
    collections::HashMap,
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::cli::ProfileName;

#[derive(Debug, Clone, Serialize)]
pub struct ApplyBody {
    pub launch_profile: Option<String>,
    pub provider: ProviderBody,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderBody {
    pub api: String,
    pub model: String,
    pub gateway_base_url: String,
    pub api_key: String,
}

pub fn build_apply_body(profile: ProfileName) -> Result<ApplyBody> {
    let dotenv = load_dotenv();
    match profile {
        ProfileName::Local => Ok(local_profile_body(&dotenv)),
        ProfileName::Deepseek => deepseek_profile_body(&dotenv),
    }
}

fn local_profile_body(dotenv: &HashMap<String, String>) -> ApplyBody {
    ApplyBody {
        launch_profile: Some(layered_or_default(
            dotenv,
            "SANTI_LAUNCH_PROFILE",
            "local-foreground",
        )),
        provider: ProviderBody {
            api: layered_or_default(dotenv, "SANTI_PROVIDER_API", "responses"),
            model: layered_or_default(dotenv, "OPENAI_MODEL", "gpt-5.4"),
            gateway_base_url: layered_or_default(
                dotenv,
                "OPENAI_BASE_URL",
                "http://127.0.0.1:18082/openai/v1",
            ),
            api_key: layered_or_default(dotenv, "OPENAI_API_KEY", "codex-local-dev"),
        },
    }
}

fn deepseek_profile_body(dotenv: &HashMap<String, String>) -> Result<ApplyBody> {
    let api_key = layered(dotenv, "DEEPSEEK_API_KEY")
        .or_else(|| shell_printenv("DEEPSEEK_API_KEY"))
        .ok_or_else(|| {
            anyhow!(
                "missing DEEPSEEK_API_KEY for deepseek profile (checked process env, root .env, launchctl, and shell rc)"
            )
        })?;
    Ok(ApplyBody {
        launch_profile: Some(layered_or_default(
            dotenv,
            "SANTI_LAUNCH_PROFILE",
            "local-foreground-deepseek",
        )),
        provider: ProviderBody {
            api: layered_or_default(dotenv, "SANTI_PROVIDER_API", "chat-completions"),
            model: layered_or_default(dotenv, "DEEPSEEK_MODEL", "deepseek-chat"),
            gateway_base_url: layered_or_default(
                dotenv,
                "DEEPSEEK_BASE_URL",
                "https://api.deepseek.com",
            ),
            api_key,
        },
    })
}

fn layered_or_default(dotenv: &HashMap<String, String>, key: &str, default: &str) -> String {
    layered(dotenv, key).unwrap_or_else(|| default.to_string())
}

fn layered(dotenv: &HashMap<String, String>, key: &str) -> Option<String> {
    non_empty(env::var(key).ok())
        .or_else(|| non_empty(dotenv.get(key).cloned()))
        .or_else(|| launchctl_get(key))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let v = v.trim().to_string();
        if v.is_empty() { None } else { Some(v) }
    })
}

fn launchctl_get(key: &str) -> Option<String> {
    if std::env::consts::OS != "macos" {
        return None;
    }
    let output = Command::new("launchctl")
        .args(["getenv", key])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn shell_printenv(key: &str) -> Option<String> {
    if !is_valid_env_key(key) {
        return None;
    }
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    for flag in &["-lc", "-ic"] {
        if let Some(value) = try_shell_printenv(&shell, flag, key) {
            return Some(value);
        }
    }
    None
}

fn try_shell_printenv(shell: &str, flag: &str, key: &str) -> Option<String> {
    let output = Command::new(shell)
        .args([flag, "printenv \"$1\"", "santi-cli-env", key])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let first = stdout.lines().next()?.trim().to_string();
    if first.is_empty() { None } else { Some(first) }
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn load_dotenv() -> HashMap<String, String> {
    let Some(path) = find_dotenv() else {
        return HashMap::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    parse_dotenv(&content)
}

fn find_dotenv() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("SANTI_CLI_ENV_FILE") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Some(path);
        }
    }
    let mut cursor = env::current_dir().ok()?;
    for _ in 0..10 {
        let candidate = cursor.join(".env");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    None
}

fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim();
        if !is_valid_env_key(key) {
            continue;
        }
        out.insert(key.to_string(), unquote(line[eq + 1..].trim()));
    }
    out
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if first == last && (first == b'"' || first == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotenv_parser_handles_quotes_export_and_comments() {
        let content = "
# top comment
OPENAI_MODEL=gpt-5.4
export SANTI_PROVIDER_API='responses'
OPENAI_BASE_URL=\"http://127.0.0.1:18082/openai/v1\"
not a key
";
        let parsed = parse_dotenv(content);
        assert_eq!(
            parsed.get("OPENAI_MODEL").map(String::as_str),
            Some("gpt-5.4")
        );
        assert_eq!(
            parsed.get("SANTI_PROVIDER_API").map(String::as_str),
            Some("responses")
        );
        assert_eq!(
            parsed.get("OPENAI_BASE_URL").map(String::as_str),
            Some("http://127.0.0.1:18082/openai/v1")
        );
    }

    #[test]
    fn local_profile_defaults_match_local_foreground_shape() {
        let body = local_profile_body(&HashMap::new());
        assert_eq!(body.launch_profile.as_deref(), Some("local-foreground"));
        assert_eq!(body.provider.api, "responses");
        assert_eq!(body.provider.model, "gpt-5.4");
        assert_eq!(
            body.provider.gateway_base_url,
            "http://127.0.0.1:18082/openai/v1"
        );
        assert_eq!(body.provider.api_key, "codex-local-dev");
    }

    #[test]
    fn local_profile_dotenv_overrides_defaults() {
        let mut dotenv = HashMap::new();
        dotenv.insert("OPENAI_MODEL".to_string(), "gpt-test".to_string());
        let body = local_profile_body(&dotenv);
        assert_eq!(body.provider.model, "gpt-test");
    }

    #[test]
    fn deepseek_profile_requires_api_key() {
        let dotenv = HashMap::new();
        unsafe {
            env::remove_var("DEEPSEEK_API_KEY");
            env::set_var("SHELL", "/usr/bin/false");
        }
        let err = deepseek_profile_body(&dotenv).unwrap_err();
        assert!(err.to_string().contains("DEEPSEEK_API_KEY"));
    }

    #[test]
    fn is_valid_env_key_rejects_bad_first_char() {
        assert!(is_valid_env_key("FOO"));
        assert!(is_valid_env_key("_FOO"));
        assert!(is_valid_env_key("FOO_1"));
        assert!(!is_valid_env_key("1FOO"));
        assert!(!is_valid_env_key("FOO-BAR"));
        assert!(!is_valid_env_key(""));
    }
}
