//! What every client of a PluSystemMonitor daemon shares: the connection
//! (TLS with pinned certificates), the data sources made from samples,
//! virtual sensors and the machines the app has connected to. Used by the
//! app and by the daemon's widget service.

pub mod apps;
pub mod client;
pub mod machines;
pub mod metrics;
pub mod paths;
pub mod processes;
pub mod tls;
pub mod virtual_sensors;
