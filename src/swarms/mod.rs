// ============================================================================
// swarms/mod.rs — Swarm templates registry
// ============================================================================

pub mod audit;
pub mod dev;
pub mod triage;

pub use audit::create_audit_swarm;
pub use dev::create_dev_swarm;
pub use triage::create_triage_swarm;
