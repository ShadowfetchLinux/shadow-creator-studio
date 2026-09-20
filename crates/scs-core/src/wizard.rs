use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WizardState {
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

impl WizardState {
    pub fn mark_completed() -> Self {
        Self {
            completed: true,
            completed_at: Some(Utc::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_completed_sets_timestamp() {
        let w = WizardState::mark_completed();
        assert!(w.completed);
        assert!(w.completed_at.is_some());
    }
}
