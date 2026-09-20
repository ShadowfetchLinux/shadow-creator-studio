use serde::{Deserialize, Serialize};

use scs_core::RecordingMetadata;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryIndex {
    pub recordings: Vec<RecordingMetadata>,
}

impl LibraryIndex {
    pub fn is_empty(&self) -> bool {
        self.recordings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_index() {
        assert!(LibraryIndex::default().is_empty());
    }
}
