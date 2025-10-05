use anyhow::{Result, anyhow, bail, ensure};
use clap::{Parser, Subcommand, ValueEnum};
use niri_ipc::{Action, PositionChange, Request, socket::Socket};
use std::process::Stdio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Snap (move) floating windows by given direction or run given action.
    FloatingSnapOr {
        /// Direction to move the floating window
        #[arg(short, long, value_parser)]
        direction: Direction,
        /// If the focusing window is not floating, then run this action
        #[command(subcommand)]
        or_action: Action,
    },
    /// Toggle window follow mode, only when the focusing window is floating.
    ///
    /// This subcommand requires nirius
    ToggleFollowMode,
    ConsumeIntoLeft,
}

#[derive(Debug, Clone, ValueEnum)]
enum Direction {
    Left,
    Down,
    Up,
    Right,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::FloatingSnapOr {
            direction,
            or_action,
        } => {
            let mut socket = Socket::connect()?;

            let niri_ipc::Response::FocusedWindow(Some(window)) = socket
                .send(Request::FocusedWindow)?
                .map_err(|e| anyhow!("{e}"))?
            else {
                bail!("failed to receive response")
            };

            if !window.is_floating {
                socket
                    .send(Request::Action(or_action))?
                    .map_err(|e| anyhow!("{e}"))?;
            } else {
                let niri_ipc::Response::FocusedOutput(Some(output)) = socket
                    .send(Request::FocusedOutput)?
                    .map_err(|e| anyhow!("{e}"))?
                else {
                    bail!("failed to receive response")
                };

                const LEFT_MARGIN: f64 = 0.;
                const BOTTOM_MARGIN: f64 = 48.;
                const TOP_MARGIN: f64 = 0.;
                const RIGHT_MARGIN: f64 = 0.;

                let (x, y): (Option<f64>, Option<f64>) = match direction {
                    Direction::Left => (Some(LEFT_MARGIN), None),
                    Direction::Down => (
                        None,
                        Some(
                            output.logical.map(|l| l.height as f64).unwrap_or_default()
                                - BOTTOM_MARGIN
                                - window.layout.tile_size.1,
                        ),
                    ),
                    Direction::Up => (None, Some(TOP_MARGIN)),
                    Direction::Right => (
                        Some(
                            output.logical.map(|l| l.width as f64).unwrap_or_default()
                                - RIGHT_MARGIN
                                - window.layout.tile_size.0,
                        ),
                        None,
                    ),
                };

                socket
                    .send(Request::Action(Action::MoveFloatingWindow {
                        id: Some(window.id),
                        x: x.map(|x| output.logical.map(|l| l.x as f64).unwrap_or_default() + x)
                            .map(PositionChange::SetFixed)
                            .unwrap_or(PositionChange::AdjustFixed(0.)),
                        y: y.map(|y| output.logical.map(|l| l.y as f64).unwrap_or_default() + y)
                            .map(PositionChange::SetFixed)
                            .unwrap_or(PositionChange::AdjustFixed(0.)),
                    }))?
                    .map_err(|e| anyhow!("{e}"))?;
            }
        }
        Command::ToggleFollowMode => {
            let mut socket = Socket::connect()?;

            let niri_ipc::Response::FocusedWindow(Some(window)) = socket
                .send(Request::FocusedWindow)?
                .map_err(|e| anyhow!("{e}"))?
            else {
                bail!("failed to receive response")
            };

            if window.is_floating {
                std::process::Command::new("nirius")
                    .stdout(Stdio::inherit())
                    .stdin(Stdio::inherit())
                    .arg("toggle-follow-mode")
                    .output()?;
            }
        }
        Command::ConsumeIntoLeft => {
            let mut socket = Socket::connect()?;

            let niri_ipc::Response::FocusedWindow(Some(window)) = socket
                .send(Request::FocusedWindow)?
                .map_err(|e| anyhow!("{e}"))?
            else {
                bail!("failed to receive response")
            };

            ensure!(!window.is_floating, "cannot consume a floating window");

            if let Some((in_ws, in_col)) = window.layout.pos_in_scrolling_layout {
                ensure!(
                    in_ws != 1,
                    "cannot consume a window in the first column into left"
                );

                if in_col != 1 {
                    for _ in 0..(in_col - 1) {
                        let _ = socket
                            .send(Request::Action(Action::MoveWindowUp {}))?
                            .map_err(|e| anyhow!("{e}"))?;
                    }
                }
            }

            let _ = socket
                .send(Request::Action(Action::FocusColumnLeft {}))?
                .map_err(|e| anyhow!("{e}"))?;
            let _ = socket
                .send(Request::Action(Action::ConsumeWindowIntoColumn {}))?
                .map_err(|e| anyhow!("{e}"))?;

            let _ = socket
                .send(Request::Action(Action::FocusWindow { id: window.id }))?
                .map_err(|e| anyhow!("{e}"))?;
        }
    }

    Ok(())
}
