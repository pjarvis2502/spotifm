use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SpotifmConfig {
    pub user: Option<String>,
    pub pass: Option<String>,
    pub session_cache: Option<String>,
    pub uris: Vec<String>,
    pub announce:  SpotifmAnnounceConfig,
    pub elevenlabs: SpotifmElevenLabsCfg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpotifyLoginConfig {
    Password { user: String, pass: String },
    SessionCache { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifmConfigError {
    message: String,
}

impl SpotifmConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for SpotifmConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SpotifmConfigError {}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SpotifmAnnounceConfig {
    pub song: SpotifmSongConfig,
    pub bumper: SpotifmBumperConfig,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SpotifmSongConfig {
    pub enable: bool,
    pub espeak: SpotifmEspeakCfg,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SpotifmBumperConfig {
    idx: Option<usize>,
    pub enable: bool,
    pub tags: Vec<String>,
    pub freq: usize,
    pub espeak: SpotifmEspeakCfg,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SpotifmElevenLabsCfg {
    pub key: String,
    pub voice: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SpotifmEspeakCfg {
    pub speed: u32,
    pub amplitude: u32,
    pub pitch: u32,
    pub gap: u32,
    pub voice: String,
}

impl SpotifmConfig {
    pub fn load(path: String) -> SpotifmConfig {
        let str = std::fs::read_to_string(path)
        .expect("unable to read config file");

        return SpotifmConfig::parse(&str).expect("unable to parse config file");
    }

    pub fn parse(str: &str) -> Result<SpotifmConfig, SpotifmConfigError> {
        let mut config: SpotifmConfig = serde_json::from_str(str)
            .map_err(|err| SpotifmConfigError::new(format!("unable to parse config file: {}", err)))?;

        config.announce.bumper.idx = Some(0);

        config.spotify_login_config()?;

        return Ok(config);
    }

    pub fn spotify_login_config(&self) -> Result<SpotifyLoginConfig, SpotifmConfigError> {
        match (&self.user, &self.pass, &self.session_cache) {
            (Some(user), Some(pass), _) if !user.is_empty() && !pass.is_empty() => {
                Ok(SpotifyLoginConfig::Password { user: user.clone(), pass: pass.clone() })
            }
            (_, _, Some(path)) if !path.is_empty() => {
                Ok(SpotifyLoginConfig::SessionCache { path: path.clone() })
            }
            _ => Err(SpotifmConfigError::new(
                "spotify login config requires either user/pass or session_cache"
            )),
        }
    }
}

impl SpotifmBumperConfig {
    pub fn next(&mut self) -> String {
        let tag = self.tags[self.idx.unwrap()].clone();
        self.idx = Some((self.idx.unwrap() + 1) % self.tags.len());
        return tag;
    }

    pub fn clear_tags(&mut self) {
        self.idx = Some(0);
        self.tags = Vec::new();
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn update_tags(&mut self, tags: Vec<String>) {
        self.idx = Some(0);
        self.tags = tags;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_json(auth_fields: &str) -> String {
        format!(r#"{{
            {auth_fields}
            "uris": ["spotify:playlist:test"],
            "elevenlabs": {{
                "key": "",
                "voice": ""
            }},
            "announce": {{
                "song": {{
                    "enable": false,
                    "espeak": {{
                        "gap": 10,
                        "speed": 180,
                        "pitch": 50,
                        "voice": "en-us",
                        "amplitude": 100
                    }}
                }},
                "bumper": {{
                    "enable": true,
                    "freq": 5,
                    "tags": ["station id"],
                    "espeak": {{
                        "gap": 5,
                        "speed": 170,
                        "pitch": 1,
                        "voice": "en-us",
                        "amplitude": 60
                    }}
                }}
            }}
        }}"#)
    }

    #[test]
    fn legacy_config_parses_password_login_mode() {
        let config = SpotifmConfig::parse(&config_json(r#"
            "user": "listener@example.com",
            "pass": "password",
        "#)).unwrap();

        assert_eq!(
            config.spotify_login_config().unwrap(),
            SpotifyLoginConfig::Password {
                user: "listener@example.com".to_string(),
                pass: "password".to_string(),
            }
        );
    }

    #[test]
    fn session_cache_config_parses_non_password_login_mode() {
        let config = SpotifmConfig::parse(&config_json(r#"
            "session_cache": "./spotify-session-cache",
        "#)).unwrap();

        assert_eq!(
            config.spotify_login_config().unwrap(),
            SpotifyLoginConfig::SessionCache {
                path: "./spotify-session-cache".to_string(),
            }
        );
    }

    #[test]
    fn invalid_config_without_login_mode_returns_validation_error() {
        let error = SpotifmConfig::parse(&config_json("")).unwrap_err();

        assert!(error.to_string().contains("spotify login"));
    }

    #[test]
    fn bumper_index_is_initialized_after_parsing() {
        let mut config = SpotifmConfig::parse(&config_json(r#"
            "user": "listener@example.com",
            "pass": "password",
        "#)).unwrap();

        assert_eq!(config.announce.bumper.next(), "station id");
    }
}
