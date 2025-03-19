use std::{collections::HashMap, path::PathBuf, time::Duration};

use eyre::ensure;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub displays: HashMap<String, String>,
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,

    #[serde(default)]
    pub builtin: BuiltinConfig,
    #[serde(default)]
    pub animations: HashMap<String, AnimationConfig>,
}

fn default_socket_path() -> PathBuf {
    "/run/fw-lights.sock".into()
}

impl Config {
    pub fn validate(&self) -> eyre::Result<()> {
        if let Some(charger) = &self.builtin.charger {
            for animation in [&charger.animation_left, &charger.animation_right] {
                ensure!(
                    self.animations.contains_key(animation),
                    "animation `{}` specified for `builtin.charger` does not exist",
                    animation
                );
            }
            ensure!(
                self.displays.contains_key(&charger.left_display),
                "display `{}` specified for `builtin.charger does not exist",
                charger.left_display
            );
            ensure!(
                self.displays.contains_key(&charger.right_display),
                "display `{}` specified for `builtin.charger does not exist",
                charger.right_display
            );
        }
        Ok(())
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct BuiltinConfig {
    pub charger: Option<ChargerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ChargerConfig {
    pub animation_left: String,
    pub animation_right: String,
    #[serde(default)]
    pub offset: i8,
    #[serde(default = "default_left_display")]
    pub left_display: String,
    #[serde(default = "default_right_display")]
    pub right_display: String,
}

fn default_left_display() -> String {
    "left".into()
}

fn default_right_display() -> String {
    "right".into()
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AnimationConfig {
    Builtin(BuiltinAnimation),
    File(FileAnimation),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "name", rename_all = "lowercase")]
pub enum BuiltinAnimation {
    Spread(SpreadAnimation),
}

#[derive(Clone, Debug, Deserialize)]
pub struct SpreadAnimation {
    pub seeds: Vec<[u8; 3]>,
    #[serde(with = "humantime_serde")]
    pub frame_duration: Duration,
    pub stay_cost: u8,
    pub horiz_cost: u8,
    pub vert_cost: u8,
    pub diag_cost: u8,
}

#[derive(Debug, Deserialize)]
pub struct FileAnimation {
    pub path: PathBuf,
}
