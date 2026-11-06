#![cfg(feature = "std")]

use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("local failure"))]
struct LocalError {
    state: Rc<Cell<u8>>,
}

#[suzunari_error]
#[suzu(display("local shutdown failure"))]
struct LocalShutdownError {
    source: Box<dyn StackError>,
}

#[suzunari_error]
#[suzu(display("borrowed value: {value}"))]
struct BorrowedError<'a> {
    value: &'a str,
}

#[suzunari_error]
enum LocalEnumError {
    #[suzu(display("local enum failure"))]
    Failed { state: Rc<Cell<u8>> },
}

// Exercise the derive entry point independently of the attribute macro.
#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("direct local failure"))]
struct DirectLocalError {
    state: Rc<Cell<u8>>,
    #[snafu(implicit)]
    location: Location,
}

#[suzunari_error]
#[suzu(display("sendable failure: {value}"))]
struct SendableError {
    value: u8,
}

#[suzunari_error]
#[suzu(display("explicit boxing"))]
struct ExplicitBoxingError {}

#[suzunari_error]
#[suzu(display("non-sync failure"))]
struct NonSyncError {
    state: Cell<u8>,
}

#[suzunari_error]
#[suzu(display("generic failure"))]
struct GenericError<T>
where
    T: std::fmt::Debug,
{
    state: T,
}

#[suzunari_error]
#[suzu(display("phantom failure"))]
struct PhantomError<T: std::fmt::Debug, const N: usize> {
    marker: PhantomData<fn() -> T>,
    bytes: [u8; N],
}

#[test]
fn a_concrete_local_error_preserves_its_shared_state_without_send_or_sync() {
    let state = Rc::new(Cell::new(3));
    let error = None::<()>
        .context(LocalSnafu {
            state: Rc::clone(&state),
        })
        .unwrap_err();
    error.state.set(7);
    assert_eq!(state.get(), 7);
    assert_eq!(error.type_name(), "LocalError");
    assert_eq!(error.to_string(), "local failure");
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn a_local_trait_object_remains_a_typed_error_source() {
    let state = Rc::new(Cell::new(3));
    let source = None::<()>.context(LocalSnafu { state }).unwrap_err();
    let source: Box<dyn StackError> = Box::new(source);
    let error = Err::<(), _>(source)
        .context(LocalShutdownSnafu)
        .unwrap_err();
    assert_eq!(error.to_string(), "local shutdown failure");
    assert_eq!(error.stack_source().unwrap().type_name(), "LocalError");
    assert_eq!(
        std::error::Error::source(&error).unwrap().to_string(),
        "local failure"
    );
}

#[test]
fn error_context_can_borrow_non_static_data() {
    let value = String::from("application-local");
    let error = None::<()>
        .context(BorrowedSnafu {
            value: value.as_str(),
        })
        .unwrap_err();
    assert_eq!(error.value, value);
    assert_eq!(error.to_string(), "borrowed value: application-local");
    assert_eq!(error.type_name(), "BorrowedError");
}

#[test]
fn a_concrete_local_enum_preserves_its_payload() {
    let state = Rc::new(Cell::new(5));
    let error = None::<()>
        .context(FailedSnafu {
            state: Rc::clone(&state),
        })
        .unwrap_err();
    let LocalEnumError::Failed {
        state: retained, ..
    } = &error;
    assert!(Rc::ptr_eq(retained, &state));
    assert_eq!(error.type_name(), "LocalEnumError::Failed");
}

#[test]
fn the_derive_entry_point_allows_a_concrete_local_error() {
    let state = Rc::new(Cell::new(9));
    let error = None::<()>
        .context(DirectLocalSnafu {
            state: Rc::clone(&state),
        })
        .unwrap_err();
    assert!(Rc::ptr_eq(&error.state, &state));
    assert_eq!(error.to_string(), "direct local failure");
    assert_eq!(error.type_name(), "DirectLocalError");
}

#[test]
fn sendable_errors_convert_automatically() {
    let error = None::<()>
        .context(SendableSnafu { value: 4_u8 })
        .unwrap_err();
    let boxed: BoxedStackError = error.into();
    assert_eq!(boxed.type_name(), "SendableError");
    assert_eq!(boxed.to_string(), "sendable failure: 4");
}

#[test]
fn explicit_boxing_remains_available() {
    let error = None::<()>.context(ExplicitBoxingSnafu).unwrap_err();
    let boxed = BoxedStackError::new(error);
    assert_eq!(boxed.type_name(), "ExplicitBoxingError");
}

#[test]
fn a_sendable_non_sync_error_remains_usable() {
    fn requires_send<T: Send>(_: &T) {}

    let error = None::<()>
        .context(NonSyncSnafu {
            state: Cell::new(3),
        })
        .unwrap_err();
    requires_send(&error);
    error.state.set(8);
    assert_eq!(error.state.get(), 8);
    assert_eq!(error.type_name(), "NonSyncError");
}

#[test]
fn a_generic_error_can_hold_local_state() {
    let state = Rc::new(Cell::new(2));
    let error: GenericError<Rc<Cell<u8>>> = None::<()>
        .context(GenericSnafu {
            state: Rc::clone(&state),
        })
        .unwrap_err();
    error.state.set(6);
    assert_eq!(state.get(), 6);
    assert_eq!(error.type_name(), "GenericError");
}

#[test]
fn a_generic_sendable_error_propagates_into_a_boxed_result() {
    fn operation() -> Result<(), BoxedStackError> {
        let result: Result<(), GenericError<u8>> = None::<()>.context(GenericSnafu { state: 4_u8 });
        result?;
        Ok(())
    }

    let error = operation().unwrap_err();
    assert_eq!(error.type_name(), "GenericError");
    assert_eq!(error.to_string(), "generic failure");
    assert_eq!(error.location().file(), file!());
}

#[test]
fn a_static_borrowing_error_converts_automatically() {
    let error = None::<()>
        .context(BorrowedSnafu { value: "static" })
        .unwrap_err();
    let boxed: BoxedStackError = error.into();
    assert_eq!(boxed.type_name(), "BorrowedError");
    assert_eq!(boxed.to_string(), "borrowed value: static");
}

#[test]
fn conversion_requires_auto_traits_of_the_error_not_its_type_parameters() {
    let error: PhantomError<Rc<()>, 3> = None::<()>
        .context(PhantomSnafu {
            marker: PhantomData,
            bytes: [1, 2, 3],
        })
        .unwrap_err();
    assert_eq!(error.bytes, [1, 2, 3]);
    let boxed: BoxedStackError = error.into();
    assert_eq!(boxed.type_name(), "PhantomError");
    assert_eq!(boxed.to_string(), "phantom failure");
}

#[test]
fn identity_and_raw_box_conversions_preserve_the_original_allocation() {
    fn convert(error: impl Into<BoxedStackError>) -> BoxedStackError {
        error.into()
    }

    let error = None::<()>
        .context(SendableSnafu { value: 9_u8 })
        .unwrap_err();
    let boxed: BoxedStackError = error.into();
    let original = std::ptr::from_ref(boxed.inner()).cast::<()>();
    let same = convert(boxed);
    assert_eq!(original, std::ptr::from_ref(same.inner()).cast::<()>());
    let raw: Box<dyn StackError + Send + Sync> = same.into();
    let restored: BoxedStackError = raw.into();
    assert_eq!(original, std::ptr::from_ref(restored.inner()).cast::<()>());
    assert_eq!(restored.type_name(), "SendableError");
    assert_eq!(restored.to_string(), "sendable failure: 9");
}

#[cfg(feature = "serde")]
#[test]
fn automatic_conversion_combines_with_serialization() {
    #[suzunari_error(serialize)]
    #[suzu(display("serializable failure: {value}"))]
    struct SerializableError {
        value: u8,
    }
    let error = None::<()>
        .context(SerializableSnafu { value: 8_u8 })
        .unwrap_err();
    let location = error.location();
    let mut expected = serde_json::json!({
        "type": "SerializableError",
        "message": "serializable failure: 8",
        "location": {
            "file": location.file(),
            "line": location.line(),
            "column": location.column(),
        },
        "context": {"value": 8},
    });
    assert_eq!(serde_json::to_value(&error).unwrap(), expected);
    let boxed: BoxedStackError = error.into();
    expected.as_object_mut().unwrap().remove("context");
    assert_eq!(serde_json::to_value(boxed).unwrap(), expected);
}
