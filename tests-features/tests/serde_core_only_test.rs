//! The serde feature under each tier, and what only the core-only one can show.
//!
//! The comparison is the one the main suite makes — the generated definition
//! against a struct written by hand — but carried out by writing both into
//! fixed buffers and comparing the bytes, since the recording serializer needs
//! an allocator.
//!
//! Running here at all is the point, and what it proves is a compile-time
//! property rather than a runtime one. With serde taken without default
//! features, `collect_str` has no default implementation — the one that goes
//! through `to_string()` — so a serializer must write the value itself. That
//! this compiles and runs means nothing on the path to `message` reached for
//! that default. The test binary still links std for its harness, so an
//! allocator does exist in the process; the claim is about which code exists,
//! not about what the process can do.

#![no_std]
#![cfg(all(feature = "test-serde", not(feature = "test-alloc")))]

use suzunari_error::*;
use suzunari_error_feature_tests::canonical::{Error, canonical};

#[suzunari_error(serialize)]
#[suzu(display("lookup failed for {key}"))]
struct LookupError {
    key: &'static str,
    attempts: u32,
}

/// What a user would have written for the declared fields.
mod oracle {
    #[derive(serde::Serialize)]
    pub struct LookupError {
        pub key: &'static str,
        pub attempts: u32,
    }
}

fn lookup_error() -> LookupError {
    fn failing() -> Result<(), LookupError> {
        ensure!(
            false,
            LookupSnafu {
                key: "k",
                attempts: 3u32,
            }
        );
        Ok(())
    }
    failing().unwrap_err()
}

/// The whole payload, so that `message` — and with it `collect_str` — is on the
/// path.
#[test]
fn test_the_payload_is_written_without_an_allocator() {
    let mut bytes = [0u8; 512];
    let written = canonical(&mut bytes, &lookup_error()).expect("fits");
    let text = core::str::from_utf8(written).expect("utf-8");

    assert!(text.starts_with("S\"StackError\"(4){"), "{text}");
    assert!(text.contains("type=\"LookupError\""), "{text}");
    assert!(text.contains("message=\"lookup failed for k\""), "{text}");
    assert!(text.contains("context=S\"LookupError\"(2){"), "{text}");
    // No cause, so the field is announced as skipped rather than written.
    assert!(text.contains("-source"), "{text}");
}

/// The same comparison the other tiers make, byte for byte.
#[test]
fn test_context_matches_a_directly_written_struct() {
    let error = lookup_error();
    let expected = oracle::LookupError {
        key: "k",
        attempts: 3,
    };

    let mut theirs = [0u8; 256];
    let theirs = canonical(&mut theirs, &expected).expect("fits");

    let mut ours = [0u8; 512];
    let ours = canonical(&mut ours, &error).expect("fits");
    let ours = core::str::from_utf8(ours).expect("utf-8");
    let theirs = core::str::from_utf8(theirs).expect("utf-8");

    let context = ours
        .split_once("context=")
        .expect("a context")
        .1
        .strip_suffix(",-source}")
        .expect("the tail");
    assert_eq!(context, theirs);
}

/// Overflow is an error, not a short write.
///
/// Two values that both ran out of room would otherwise produce the same
/// truncated bytes and compare equal, which would leave the comparison passing
/// on nothing.
#[test]
fn test_a_buffer_that_is_too_small_fails() {
    let mut bytes = [0u8; 8];
    assert_eq!(
        canonical(&mut bytes, &lookup_error()).unwrap_err(),
        Error::Overflow
    );
}
