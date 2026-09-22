//! Preserve an ambiguous setup failure instead of implicitly resubmitting.
pub fn capture_setup_failure<T>(
    setup: impl FnOnce() -> T + std::panic::UnwindSafe,
) -> Result<T, String> {
    std::panic::catch_unwind(setup).map_err(|failure| {
        failure
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| failure.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "non-string setup panic".into())
    })
}

#[test]
fn failed_setup_is_cached_without_resubmission() {
    use once_cell::sync::OnceCell;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let attempts = AtomicUsize::new(0);
    let cell: OnceCell<Result<(), String>> = OnceCell::new();
    for _ in 0..3 {
        let result = cell.get_or_init(|| {
            capture_setup_failure(|| {
                attempts.fetch_add(1, Ordering::SeqCst);
                panic!("ambiguous transaction outcome");
            })
        });
        assert_eq!(
            result.as_ref().unwrap_err(),
            "ambiguous transaction outcome"
        );
    }
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}
