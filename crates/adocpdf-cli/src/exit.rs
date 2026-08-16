//! Mapping failures to exit codes.
//!
//! Each class of failure gets its own code so a script can tell them apart
//! without parsing messages. The numbers follow the `sysexits.h` convention
//! where one fits, because that is what other command-line tools do and there
//! is nothing to gain from inventing a private scheme.

use adocpdf_domain::error::DomainError;

/// The command line itself was wrong.
pub(crate) const USAGE: u8 = 64;

/// The input could not be read, or is not usable.
pub(crate) const DATA_ERROR: u8 = 65;

/// A named input does not exist.
pub(crate) const NO_INPUT: u8 = 66;

/// A path was refused by the sandbox.
pub(crate) const NO_PERMISSION: u8 = 77;

/// The output could not be written.
pub(crate) const CANNOT_CREATE: u8 = 73;

/// Something internal went wrong — here, laying the document out.
pub(crate) const SOFTWARE: u8 = 70;

/// The exit code for a failure.
#[must_use]
pub(crate) fn code_for(error: &DomainError) -> u8 {
    match error {
        DomainError::InputNotFound { .. } => NO_INPUT,

        DomainError::InputUnreadable { .. }
        | DomainError::ParseFailed { .. }
        | DomainError::InvalidTheme { .. }
        | DomainError::UnknownTheme { .. } => DATA_ERROR,

        DomainError::OutputUnwritable { .. } => CANNOT_CREATE,

        DomainError::LayoutFailed { .. } => SOFTWARE,

        DomainError::PathOutsideRoot { .. }
        | DomainError::ReferenceOutsideRoot { .. }
        | DomainError::RootNotADirectory { .. } => NO_PERMISSION,
    }
}

#[cfg(test)]
mod tests {
    use adocpdf_domain::error::SourceLocation;

    use super::*;

    fn every_failure_class() -> Vec<DomainError> {
        vec![
            DomainError::InputNotFound {
                path: "in.adoc".to_owned(),
            },
            DomainError::InputUnreadable {
                path: "in.adoc".to_owned(),
                reason: "busy".to_owned(),
            },
            DomainError::ParseFailed {
                path: "in.adoc".to_owned(),
                location: SourceLocation::START,
                reason: "bad".to_owned(),
            },
            DomainError::OutputUnwritable {
                path: "out.pdf".to_owned(),
                reason: "denied".to_owned(),
            },
            DomainError::LayoutFailed {
                path: "in.adoc".to_owned(),
                reason: "cannot fit".to_owned(),
            },
            DomainError::InvalidTheme {
                id: "narrow".to_owned(),
                reason: "no printable width".to_owned(),
            },
            DomainError::UnknownTheme {
                id: "absent".to_owned(),
                section: "Appendix".to_owned(),
            },
            DomainError::PathOutsideRoot {
                requested: "x".to_owned(),
                root: "/p".to_owned(),
            },
            DomainError::ReferenceOutsideRoot {
                reference: "include::x[]".to_owned(),
                location: SourceLocation::START,
                path: "in.adoc".to_owned(),
                root: "/p".to_owned(),
            },
            DomainError::RootNotADirectory {
                root: "/nowhere".to_owned(),
            },
        ]
    }

    #[test]
    fn a_missing_input_is_distinct_from_an_unreadable_one() {
        assert_ne!(
            code_for(&DomainError::InputNotFound {
                path: "in.adoc".to_owned()
            }),
            code_for(&DomainError::InputUnreadable {
                path: "in.adoc".to_owned(),
                reason: "busy".to_owned(),
            })
        );
    }

    #[test]
    fn writing_parsing_and_laying_out_fail_with_different_codes() {
        let write = code_for(&DomainError::OutputUnwritable {
            path: "out.pdf".to_owned(),
            reason: "denied".to_owned(),
        });
        let parse = code_for(&DomainError::ParseFailed {
            path: "in.adoc".to_owned(),
            location: SourceLocation::START,
            reason: "bad".to_owned(),
        });
        let layout = code_for(&DomainError::LayoutFailed {
            path: "in.adoc".to_owned(),
            reason: "cannot fit".to_owned(),
        });

        assert_ne!(write, parse);
        assert_ne!(write, layout);
        assert_ne!(parse, layout);
    }

    #[test]
    fn every_confinement_failure_shares_one_code() {
        let outside = code_for(&DomainError::PathOutsideRoot {
            requested: "x".to_owned(),
            root: "/p".to_owned(),
        });
        let root = code_for(&DomainError::RootNotADirectory {
            root: "/nowhere".to_owned(),
        });

        assert_eq!(
            outside, root,
            "a caller only needs to know the sandbox refused it"
        );
        assert_eq!(outside, NO_PERMISSION);
    }

    #[test]
    fn no_failure_reports_success() {
        for error in every_failure_class() {
            assert_ne!(
                code_for(&error),
                0,
                "{error} must not be mistaken for success"
            );
        }
    }

    #[test]
    fn usage_is_distinct_from_every_render_failure() {
        for error in every_failure_class() {
            assert_ne!(code_for(&error), USAGE, "{error} is not a usage problem");
        }
    }
}
