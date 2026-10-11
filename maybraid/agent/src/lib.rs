//! Headless agent drive / observe: scripted pad input and JSONL events.
//!
//! This crate is Bevy-free so parse and playback tests stay cheap. The world
//! plugin applies [`PadIntent`] to the virtual-pad overlay and writes
//! [`AgentEvent`]s.

pub mod cli;
pub mod command;
pub mod event;
pub mod io;
pub mod pad;
pub mod playback;
pub mod script;
pub mod sink;

pub use cli::{
	take_agent_args, AgentArgs, AGENT_EVENTS_ENV, AGENT_PIPE_ENV, AGENT_QUIT_ENV, AGENT_SCRIPT_ENV,
};
pub use command::{AgentButton, AgentCommand, ScenarioMark};
pub use event::{AgentEvent, AgentEventBody, CommandStatus};
pub use io::LineInbox;
pub use pad::PadIntent;
pub use playback::{Playback, Tick, TickEffect, TickView};
pub use script::read_script_commands;
pub use sink::EventSink;
