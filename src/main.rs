use std::convert::Infallible;

use eyre::eyre;
use fw_lights::{config::Config, daemon};

fn main() -> eyre::Result<Infallible> {
    color_eyre::install()?;

    let config_path = std::env::args()
        .nth(1)
        .ok_or_else(|| eyre!("please pass config path"))?;
    let raw_config = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&raw_config)?;
    daemon::run(config)
}
