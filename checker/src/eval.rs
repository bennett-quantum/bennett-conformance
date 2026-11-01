//! Replay claims (exhaustive / sampled / inverse_pair). Implemented in the
//! replay slice; this placeholder keeps every claim honestly unchecked.

use crate::cert::{ClaimOutcome, Inputs, Invocation, LoadedDocs};
use crate::json::Value;
use crate::object::Symbol;

#[allow(clippy::too_many_arguments)]
pub fn check_replay_claim(
    _claim: &Value,
    _path: &str,
    _region_hash: &str,
    _region_symbol: Option<&Symbol>,
    _docs: &LoadedDocs,
    _root_invocation: &Option<Invocation>,
    _cert_schedule_hash: Option<&str>,
    _inputs: &Inputs,
) -> ClaimOutcome {
    ClaimOutcome::unchecked("replay checker not yet implemented in this slice")
}
