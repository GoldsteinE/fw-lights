use std::{
    collections::HashMap,
    convert::Infallible,
    io::{BufRead as _, BufReader, Write as _},
    os::unix::net::UnixListener,
    str::FromStr as _,
    sync::{
        Arc,
        atomic::{self, AtomicU64},
    },
    thread,
    time::{Duration, Instant},
};

use framework_lib::power::UsbPowerRoles;

use crate::{
    MatrixPort,
    animations::builder::AnimationBuilder,
    config::Config,
    display_thread::{self, DisplayCommand},
};

pub fn run(config: Config) -> eyre::Result<Infallible> {
    config.validate()?;

    let ec = Arc::new(framework_lib::chromium_ec::CrosEc::new());

    let builtin_config = Arc::new(config.builtin);

    let animations = Arc::new(
        config
            .animations
            .into_iter()
            .map(|(name, config)| AnimationBuilder::new(config).map(|builder| (name, builder)))
            .collect::<eyre::Result<HashMap<_, _>>>()?,
    );
    let displays = Arc::new(
        config
            .displays
            .into_iter()
            .map(|(name, path)| {
                Ok((
                    name,
                    display_thread::Matrix::new(MatrixPort::open(&path)?)?.spawn(),
                ))
            })
            .collect::<eyre::Result<HashMap<_, _>>>()?,
    );

    let reference = Instant::now();
    // TODO: this should really be replaced with some sort of "already-playing" detection
    let charger_last_played = Arc::new(AtomicU64::new(reference.elapsed().as_millis() as u64));

    let socket = UnixListener::bind(&config.socket_path)?;

    loop {
        let (stream, _addr) = socket.accept()?;
        let ec = Arc::clone(&ec);
        let builtin_config = Arc::clone(&builtin_config);
        let animations = Arc::clone(&animations);
        let displays = Arc::clone(&displays);
        let charger_last_played = Arc::clone(&charger_last_played);
        thread::spawn(move || -> eyre::Result<()> {
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            loop {
                line.clear();
                if stream.read_line(&mut line)? == 0 {
                    return Ok(());
                }
                // not easy to avoid this allocation due to borrow checker
                let words: Vec<_> = line.split_ascii_whitespace().collect();
                match words.as_slice() {
                    ["charger"] => {
                        let now = reference.elapsed().as_millis() as u64;
                        let throttled = charger_last_played
                            .fetch_update(
                                atomic::Ordering::Relaxed,
                                atomic::Ordering::Relaxed,
                                |old| {
                                    if now - old > 1000 { Some(now) } else { None }
                                },
                            )
                            .is_err();
                        if throttled {
                            stream.get_mut().write_all(b"OK throttled\n")?;
                            continue;
                        }

                        let Some(config) = &builtin_config.charger else {
                            stream.get_mut().write_all(b"ERR no config")?;
                            continue;
                        };

                        for (idx, port) in framework_lib::power::get_pd_info(&ec, 4)
                            .into_iter()
                            .enumerate()
                        {
                            if let Ok(port) = port {
                                if matches!(port.role, UsbPowerRoles::Sink) {
                                    let (side, animation, offset) = match idx {
                                        0 => (&config.right_display, &config.animation_right, 14),
                                        1 => (&config.right_display, &config.animation_right, 24),
                                        2 => (&config.left_display, &config.animation_left, 24),
                                        3 => (&config.left_display, &config.animation_left, 14),
                                        // unknown port
                                        _ => continue,
                                    };
                                    // already validated
                                    let display = &displays[side].0;
                                    let animation =
                                        animations[animation].at(offset + config.offset);
                                    display.send(DisplayCommand::AddAnimation(animation))?;
                                }
                            }
                        }

                        stream.get_mut().write_all(b"OK\n")?;
                    }
                    &["play", animation, "at", display, ref args @ ..] => {
                        let Some((display, _thread)) = displays.get(display) else {
                            stream.get_mut().write_all(b"ERR bad display\n")?;
                            continue;
                        };
                        let Some(animation_builder) = animations.get(animation) else {
                            stream.get_mut().write_all(b"ERR bad animation\n")?;
                            continue;
                        };
                        let animation = match args {
                            [] => animation_builder.build(),
                            ["offset", offset] => {
                                let Ok(offset) = i8::from_str(offset) else {
                                    stream.get_mut().write_all(b"ERR bad offset\n")?;
                                    continue;
                                };
                                animation_builder.at(offset)
                            }
                            _ => {
                                stream.get_mut().write_all(b"ERR bad args\n")?;
                                continue;
                            }
                        };
                        display.send(DisplayCommand::AddAnimation(animation))?;
                        stream.get_mut().write_all(b"OK\n")?;
                    }
                    _ => {
                        stream.get_mut().write_all(b"ERR unknown command\n")?;
                    }
                }
            }
        });
    }
}
