// TODO's:
// * add more consistency to error handling
// * decide on a more consistent separation between windows and clients
// * clean up code

#[macro_use]
pub mod util;

pub mod client;
pub mod config;
pub mod err;
pub mod kbd;
pub mod layout;
pub mod window_system;
