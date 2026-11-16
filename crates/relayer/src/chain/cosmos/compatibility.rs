//! Cosmos-SDK compatibility constants and diagnostic methods.

use thiserror::Error;
use tracing::debug;

use super::version;
use crate::chain::version::ConsensusVersion;

/// Specifies the SDK module version requirement.
///
/// # Note: Should be consistent with [features] guide page.
///
/// [features]: https://hermes.informal.systems/advanced/features.html
const SDK_MODULE_VERSION_REQ: &str = ">=0.46, <0.54";

/// Specifies the IBC-go module version requirement.
/// At the moment, we support both chains with and without
/// the standalone ibc-go module, i.e., it's not an error
/// if the chain binary does not build with this module.
///
/// # Note: Should be consistent with [features] guide page.
///
/// [features]: https://hermes.informal.systems/advanced/features.html
const IBC_GO_MODULE_VERSION_REQ: &str = ">=6.3.1, <11";

#[derive(Error, Debug)]
pub enum Diagnostic {
    #[error("SDK module version not found, required {requirements}")]
    MissingSdkModuleVersion { requirements: String },

    #[error("IBC-Go module version not found, required {requirements}")]
    MissingIbcGoModuleVersion { requirements: String },

    #[error(
        "SDK module at version '{found}' does not meet compatibility requirements {requirements}"
    )]
    MismatchingSdkModuleVersion { requirements: String, found: String },

    #[error("IBC-Go module at version '{found}' does not meet compatibility requirements {requirements}")]
    MismatchingIbcGoModuleVersion { requirements: String, found: String },
}

/// Runs a diagnostic check on the provided [`VersionInfo`]
/// to ensure that the Sdk & IBC-go modules version match
/// the predefined requirements.
///
/// Returns `None` upon success, or a [`Diagnostic`] upon
/// an error.
///
/// Relies on the constant [`SDK_MODULE_NAME`] to find the
/// Sdk module by name, as well as the constants
/// [`SDK_MODULE_VERSION_REQ`] and [`IBC_GO_MODULE_VERSION_REQ`]
/// for establishing compatibility requirements.
pub(crate) fn run_diagnostic(specs: &version::Specs) -> Result<(), Diagnostic> {
    debug!("running diagnostic on version specs: {specs}");

    // The Dora Vota v1.1.0 stack has been tested for classic ICS-20 with
    // Ed25519 validator keys. Keep the exception narrow: module versions
    // alone do not establish compatibility with new validator key types.
    if is_tested_dora_stack(specs) {
        return Ok(());
    }

    sdk_diagnostic(specs.cosmos_sdk.as_ref())?;
    ibc_go_diagnostic(specs.ibc_go.as_ref())?;

    Ok(())
}

fn is_tested_dora_stack(specs: &version::Specs) -> bool {
    let matches = |v: &semver::Version, major, minor, patch| {
        (v.major, v.minor, v.patch) == (major, minor, patch)
    };
    matches!(&specs.consensus, Some(ConsensusVersion::Comet(v)) if matches(v, 0, 40, 0))
        && specs
            .cosmos_sdk
            .as_ref()
            .is_some_and(|v| matches(v, 0, 55, 0))
        && specs.ibc_go.as_ref().is_some_and(|v| matches(v, 11, 2, 0))
}

fn sdk_diagnostic(version: Option<&semver::Version>) -> Result<(), Diagnostic> {
    // Parse the SDK requirements into a semver
    let sdk_reqs = semver::VersionReq::parse(SDK_MODULE_VERSION_REQ)
        .expect("parsing the SDK module requirements into semver");

    match version {
        None => Err(Diagnostic::MissingSdkModuleVersion {
            requirements: SDK_MODULE_VERSION_REQ.to_string(),
        }),

        Some(version) => match sdk_reqs.matches(version) {
            true => Ok(()),
            false => Err(Diagnostic::MismatchingSdkModuleVersion {
                requirements: SDK_MODULE_VERSION_REQ.to_string(),
                found: version.to_string(),
            }),
        },
    }
}

fn ibc_go_diagnostic(version: Option<&semver::Version>) -> Result<(), Diagnostic> {
    // Parse the IBC-go module requirements into a semver
    let ibc_reqs = semver::VersionReq::parse(IBC_GO_MODULE_VERSION_REQ)
        .expect("parsing the IBC-Go module requirements into semver");

    match version {
        None => Err(Diagnostic::MissingIbcGoModuleVersion {
            requirements: IBC_GO_MODULE_VERSION_REQ.to_string(),
        }),
        Some(version) => match ibc_reqs.matches(version) {
            true => Ok(()),
            false => Err(Diagnostic::MismatchingIbcGoModuleVersion {
                requirements: IBC_GO_MODULE_VERSION_REQ.to_string(),
                found: version.to_string(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_tested_stack_without_accepting_other_sdk_055_stacks() {
        let mut specs = version::Specs {
            cosmos_sdk: Some(semver::Version::new(0, 55, 0)),
            ibc_go: Some(semver::Version::new(11, 2, 0)),
            consensus: Some(ConsensusVersion::Comet(semver::Version::new(0, 40, 0))),
        };
        assert!(run_diagnostic(&specs).is_ok());
        specs.ibc_go = Some(semver::Version::new(11, 1, 0));
        assert!(matches!(
            run_diagnostic(&specs),
            Err(Diagnostic::MismatchingSdkModuleVersion { .. })
        ));
    }
}
