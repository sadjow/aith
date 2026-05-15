use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

struct TestEnv {
    _temp: TempDir,
    aith_home: PathBuf,
    codex_home: PathBuf,
    claude_config_dir: PathBuf,
    home: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create temp dir");
        let root = temp.path();
        let aith_home = root.join("aith");
        let codex_home = root.join("codex");
        let claude_config_dir = root.join("claude");
        let home = root.join("home");

        fs::create_dir_all(&codex_home).expect("create fake codex home");
        fs::create_dir_all(&claude_config_dir).expect("create fake claude config dir");
        fs::create_dir_all(&home).expect("create fake home");

        Self {
            _temp: temp,
            aith_home,
            codex_home,
            claude_config_dir,
            home,
        }
    }

    fn write_auth(&self, account: &str) {
        fs::write(
            self.codex_home.join("auth.json"),
            format!("{{\"account\":\"{account}\"}}\n"),
        )
        .expect("write fake codex auth");
    }

    fn write_config(&self, config: &str) {
        fs::write(self.codex_home.join("config.toml"), config).expect("write fake codex config");
    }

    fn read_auth(&self) -> String {
        fs::read_to_string(self.codex_home.join("auth.json")).expect("read fake codex auth")
    }

    fn profile_auth(&self, profile: &str) -> PathBuf {
        self.aith_home
            .join("profiles")
            .join("codex")
            .join(profile)
            .join("auth.json")
    }

    fn claude_profile(&self, profile: &str) -> PathBuf {
        self.aith_home
            .join("profiles")
            .join("claude")
            .join(profile)
            .join("profile.toml")
    }

    fn backup_dir(&self) -> PathBuf {
        self.aith_home.join("backups").join("codex")
    }

    fn backup_ids(&self) -> Vec<String> {
        let backup_dir = self.backup_dir();
        if !backup_dir.exists() {
            return Vec::new();
        }

        let mut ids = fs::read_dir(backup_dir)
            .expect("read backups")
            .map(|entry| {
                entry
                    .expect("read backup entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();

        ids.sort();
        ids
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("aith").expect("aith binary");
        command.env("AITH_HOME", &self.aith_home);
        command.env("CODEX_HOME", &self.codex_home);
        command.env("CLAUDE_CONFIG_DIR", &self.claude_config_dir);
        command.env("HOME", &self.home);
        for name in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_VERTEX",
            "CLAUDE_CODE_USE_FOUNDRY",
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_API_KEY_WORK",
        ] {
            command.env_remove(name);
        }
        command
    }
}

#[test]
fn save_list_and_current_detect_saved_codex_profiles() {
    let env = TestEnv::new();
    env.write_auth("work");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved codex profile 'work'"));

    assert_eq!(
        fs::read_to_string(env.profile_auth("work")).expect("read saved profile"),
        "{\"account\":\"work\"}\n"
    );

    env.command()
        .args(["list", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));

    env.command()
        .args(["current", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("codex: work"));

    env.write_auth("personal");

    env.command()
        .args(["current", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("codex: unknown"));

    env.command()
        .args(["save", "codex", "personal"])
        .assert()
        .success();

    env.command()
        .args(["save", "codex", "duplicate"])
        .assert()
        .success();

    env.command()
        .args(["current", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("codex: ambiguous"))
        .stdout(predicate::str::contains("duplicate, personal"));
}

#[test]
fn use_creates_backup_and_restore_is_reversible() {
    let env = TestEnv::new();
    env.write_auth("work");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    env.write_auth("personal");

    env.command()
        .args(["use", "codex", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backup"));

    assert_eq!(env.read_auth(), "{\"account\":\"work\"}\n");

    let backups = env.backup_ids();
    assert_eq!(backups.len(), 1);

    env.command()
        .args(["backups", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&backups[0]));

    env.command()
        .args(["restore", "codex", &backups[0]])
        .assert()
        .success()
        .stdout(predicate::str::contains("restored codex backup"));

    assert_eq!(env.read_auth(), "{\"account\":\"personal\"}\n");
    assert_eq!(env.backup_ids().len(), 2);
}

#[test]
fn remove_refuses_current_profile_unless_forced() {
    let env = TestEnv::new();
    env.write_auth("work");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    env.command()
        .args(["remove", "codex", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "profile 'work' is currently active for codex",
        ));

    assert!(env.profile_auth("work").exists());

    env.command()
        .args(["remove", "codex", "work", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed codex profile 'work'"));

    assert!(!env.profile_auth("work").exists());
}

#[test]
fn exec_uses_temporary_codex_home_and_preserves_active_auth() {
    let env = TestEnv::new();
    env.write_auth("work");
    env.write_config("model = \"test-model\"\n");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    env.write_auth("personal");

    let temp_home_record = env.aith_home.join("temp-home");
    let script = format!(
        "printf '%s' \"$CODEX_HOME\" > '{}'; cat \"$CODEX_HOME/auth.json\"; cat \"$CODEX_HOME/config.toml\"",
        shell_path(&temp_home_record)
    );

    env.command()
        .args(["exec", "codex", "work", "--", "sh", "-c", &script])
        .assert()
        .success()
        .stdout(predicate::str::contains("{\"account\":\"work\"}"))
        .stdout(predicate::str::contains("model = \"test-model\""));

    assert_eq!(env.read_auth(), "{\"account\":\"personal\"}\n");

    let temp_home = fs::read_to_string(temp_home_record).expect("read temp home path");
    assert_ne!(Path::new(&temp_home), env.codex_home.as_path());
    assert!(!Path::new(&temp_home).exists());
}

#[test]
fn exec_propagates_child_exit_status() {
    let env = TestEnv::new();
    env.write_auth("work");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    env.command()
        .args(["exec", "codex", "work", "--", "sh", "-c", "exit 7"])
        .assert()
        .code(7);
}

#[test]
fn shell_uses_temporary_codex_home_and_preserves_active_auth() {
    let env = TestEnv::new();
    env.write_auth("work");
    env.write_config("model = \"test-model\"\n");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    env.write_auth("personal");

    let session_record = env.aith_home.join("shell-session");
    let fake_shell = env.aith_home.join("fake-shell");
    write_executable(
        &fake_shell,
        &format!(
            "#!/bin/sh\nprintf '%s\\n%s\\n%s\\n' \"$CODEX_HOME\" \"$AITH_TOOL\" \"$AITH_PROFILE\" > '{}'\ncat \"$CODEX_HOME/auth.json\"\ncat \"$CODEX_HOME/config.toml\"\n",
            shell_path(&session_record)
        ),
    );

    env.command()
        .env("SHELL", &fake_shell)
        .args(["shell", "codex", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("{\"account\":\"work\"}"))
        .stdout(predicate::str::contains("model = \"test-model\""));

    assert_eq!(env.read_auth(), "{\"account\":\"personal\"}\n");

    let session = fs::read_to_string(session_record).expect("read shell session record");
    let mut lines = session.lines();
    let temp_home = lines.next().expect("temp home line");
    assert_eq!(lines.next(), Some("codex"));
    assert_eq!(lines.next(), Some("work"));
    assert_ne!(Path::new(temp_home), env.codex_home.as_path());
    assert!(!Path::new(temp_home).exists());
}

#[test]
fn shell_propagates_shell_exit_status() {
    let env = TestEnv::new();
    env.write_auth("work");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    let fake_shell = env.aith_home.join("failing-shell");
    write_executable(&fake_shell, "#!/bin/sh\nexit 9\n");

    env.command()
        .env("SHELL", &fake_shell)
        .args(["shell", "codex", "work"])
        .assert()
        .code(9);
}

#[test]
fn save_list_and_remove_claude_env_profiles_without_storing_secret() {
    let env = TestEnv::new();

    env.command()
        .env("ANTHROPIC_API_KEY_WORK", "test-secret-key")
        .args([
            "save",
            "claude",
            "work",
            "--from-env",
            "ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK",
            "--set-env",
            "ANTHROPIC_BASE_URL=https://api.example.test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved claude profile 'work'"))
        .stdout(predicate::str::contains("source      environment"));

    let profile = fs::read_to_string(env.claude_profile("work")).expect("read claude profile");
    assert!(profile.contains("from_env = \"ANTHROPIC_API_KEY_WORK\""));
    assert!(profile.contains("ANTHROPIC_BASE_URL = \"https://api.example.test\""));
    assert!(!profile.contains("test-secret-key"));

    env.command()
        .args(["list", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));

    env.command()
        .args(["current", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("claude: unknown"));

    env.command()
        .args(["backups", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no claude backups saved"));

    env.command()
        .args(["remove", "claude", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed claude profile 'work'"));

    assert!(!env.claude_profile("work").exists());
}

#[test]
fn save_claude_env_profile_rejects_literal_sensitive_values() {
    let env = TestEnv::new();

    env.command()
        .args([
            "save",
            "claude",
            "bad",
            "--set-env",
            "ANTHROPIC_API_KEY=test-secret-key",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to store literal value for sensitive env ANTHROPIC_API_KEY",
        ));

    assert!(!env.claude_profile("bad").exists());
}

#[test]
fn exec_uses_claude_env_profile() {
    let env = TestEnv::new();

    env.command()
        .args([
            "save",
            "claude",
            "work",
            "--from-env",
            "ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK",
            "--set-env",
            "ANTHROPIC_BASE_URL=https://api.example.test",
        ])
        .assert()
        .success();

    env.command()
        .env("ANTHROPIC_API_KEY_WORK", "test-secret-key")
        .args([
            "exec",
            "claude",
            "work",
            "--",
            "sh",
            "-c",
            "printf '%s|%s|%s|%s' \"$ANTHROPIC_API_KEY\" \"$ANTHROPIC_BASE_URL\" \"$AITH_TOOL\" \"$AITH_PROFILE\"",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "test-secret-key|https://api.example.test|claude|work",
        ));
}

#[test]
fn exec_claude_env_profile_requires_source_env_at_runtime() {
    let env = TestEnv::new();

    env.command()
        .args([
            "save",
            "claude",
            "work",
            "--from-env",
            "ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK",
        ])
        .assert()
        .success();

    env.command()
        .args(["exec", "claude", "work", "--", "sh", "-c", "exit 0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "source env ANTHROPIC_API_KEY_WORK is not set for target env ANTHROPIC_API_KEY",
        ));
}

#[test]
fn shell_uses_claude_env_profile() {
    let env = TestEnv::new();

    env.command()
        .args([
            "save",
            "claude",
            "work",
            "--from-env",
            "ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK",
        ])
        .assert()
        .success();

    let fake_shell = env.aith_home.join("claude-shell");
    write_executable(
        &fake_shell,
        "#!/bin/sh\nprintf '%s|%s|%s' \"$ANTHROPIC_API_KEY\" \"$AITH_TOOL\" \"$AITH_PROFILE\"\n",
    );

    env.command()
        .env("SHELL", &fake_shell)
        .env("ANTHROPIC_API_KEY_WORK", "test-secret-key")
        .args(["shell", "claude", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-secret-key|claude|work"));
}

#[test]
fn doctor_reports_ready_codex_profile_state() {
    let env = TestEnv::new();
    env.write_auth("work");

    env.command()
        .args(["save", "codex", "work"])
        .assert()
        .success();

    env.command()
        .args(["doctor", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "aith store {}",
            env.aith_home.display()
        )))
        .stdout(predicate::str::contains("Codex (codex)"))
        .stdout(predicate::str::contains("auth file"))
        .stdout(predicate::str::contains("profiles          1"))
        .stdout(predicate::str::contains("backups           0"))
        .stdout(predicate::str::contains("current           work"))
        .stdout(predicate::str::contains(
            "ok                Codex profile switching is ready",
        ));
}

#[test]
fn doctor_warns_when_codex_auth_and_profiles_are_missing() {
    let env = TestEnv::new();

    env.command()
        .args(["doctor", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auth file"))
        .stdout(predicate::str::contains("missing"))
        .stdout(predicate::str::contains("profiles          0"))
        .stdout(predicate::str::contains("current           unknown"))
        .stdout(predicate::str::contains(
            "warning           active Codex auth file is missing",
        ))
        .stdout(predicate::str::contains(
            "warning           no Codex profiles are saved",
        ));
}

#[test]
fn doctor_reports_claude_env_profile_state() {
    let env = TestEnv::new();

    env.command()
        .args(["doctor", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude Code (claude)"))
        .stdout(predicate::str::contains("user config dir"))
        .stdout(predicate::str::contains("user settings"))
        .stdout(predicate::str::contains("project settings"))
        .stdout(predicate::str::contains("env ANTHROPIC_API_KEY unset"))
        .stdout(predicate::str::contains("profiles          0"))
        .stdout(predicate::str::contains("backups           0"))
        .stdout(predicate::str::contains("current           unknown"))
        .stdout(predicate::str::contains(
            "info              no Claude env profiles are saved",
        ))
        .stdout(predicate::str::contains(
            "warning           Claude global login switching is not implemented; env profiles support exec and shell only",
        ));
}

#[test]
fn doctor_reports_claude_terminal_auth_env_without_printing_secret() {
    let env = TestEnv::new();

    env.command()
        .env("ANTHROPIC_API_KEY", "test-secret-key")
        .args(["doctor", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("env ANTHROPIC_API_KEY set"))
        .stdout(predicate::str::contains(
            "info              Claude terminal auth environment is configured",
        ))
        .stdout(predicate::str::contains("test-secret-key").not());
}

fn shell_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

fn write_executable(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("script parent")).expect("create script parent");
    fs::write(path, contents).expect("write executable script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .expect("read script metadata")
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).expect("make script executable");
    }
}
