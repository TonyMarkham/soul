use std::path::PathBuf;

use crate::scan::candidate_kind::CandidateKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScanCandidate {
    pub(crate) display_path: PathBuf,
    pub(crate) kind: CandidateKind,
}
