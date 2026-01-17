//! Coordinator state definitions.

/// Coordinator operational state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatorState {
    /// Coordinator is starting up.
    Starting,
    /// Coordinator is running and accepting requests.
    Running,
    /// Coordinator is shutting down, not accepting new requests.
    ShuttingDown,
    /// Coordinator is stopped.
    Stopped,
    /// Coordinator is recovering from failure.
    Recovering,
    /// Coordinator is a follower (not accepting direct requests).
    Follower,
}

impl CoordinatorState {
    /// Check if the coordinator is operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, CoordinatorState::Running)
    }

    /// Check if the coordinator is accepting new requests.
    pub fn accepts_requests(&self) -> bool {
        matches!(self, CoordinatorState::Running)
    }

    /// Check if the coordinator is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, CoordinatorState::Stopped)
    }
}
