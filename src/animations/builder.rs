use std::{fs, str::FromStr as _};

use eyre::WrapErr as _;

use crate::{
    animations::{self, Animation},
    config::{AnimationConfig, BuiltinAnimation},
};

type BuilderFn = Box<dyn Fn(Option<i8>) -> Animation + Send + Sync>;

pub struct AnimationBuilder {
    build: Box<dyn Fn(Option<i8>) -> Animation + Send + Sync>,
}

impl AnimationBuilder {
    pub fn new(config: AnimationConfig) -> eyre::Result<Self> {
        let build = match config {
            AnimationConfig::Builtin(builtin) => match builtin {
                BuiltinAnimation::Spread(config) => Box::new(move |offset: Option<i8>| {
                    animations::spread::from_config_at(config.clone(), offset.unwrap_or(0))
                }) as BuilderFn,
            },
            AnimationConfig::File(file) => {
                let path = &file.path;
                let raw = fs::read_to_string(path).wrap_err_with(|| {
                    format!("failed to read animation file `{}`", path.display())
                })?;
                let builder = animations::file::FileAnimation::from_str(&raw)?;
                Box::new(move |offset| builder.at(offset)) as _
            }
        };

        Ok(Self { build })
    }

    pub fn build(&self) -> Animation {
        (self.build)(None)
    }

    pub fn at(&self, offset: i8) -> Animation {
        (self.build)(Some(offset))
    }
}
