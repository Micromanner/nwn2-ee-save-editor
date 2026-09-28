//! Dev-only engine oracle: the game's own rules library (`NWN2RulesLib64.dll`), driven through
//! the `toolset-oracle` helper, exposed as MCP tools. Compiled into debug builds only.

mod apply;
mod compare;
mod content;
mod runner;
mod slots;
pub mod tools;
