//! The exact, A, A′ and B helpers of parity-fixture-contract §6 (DR-B3) passing and failing on
//! constructed inputs, with the failing index and both values reported (§11).

use grid_synth::fixture::{
    compare_bitwise, compare_class_a, compare_class_a_prime, compare_class_b, compare_exact,
    compare_exact_f64, ClassBInputs, Mismatch, CLASS_A_ABS_TOL, CLASS_A_PRIME_REL_TOL,
    CLASS_B_DENSE_REL_TOL,
};

#[test]
fn exact_passes_and_fails_with_the_index() {
    let ids = [0u32, 1, 10, 100];
    assert_eq!(compare_exact(&ids, &ids).map(|r| r.n), Ok(4));
    let shifted = [0u32, 1, 100, 10];
    let m = compare_exact(&shifted, &ids).expect_err("order differs");
    assert_eq!(m.index, 2);
    assert_eq!((m.rust.as_str(), m.oracle.as_str()), ("100", "10"));
    // Sentinels and empty-versus-missing are exact comparisons too.
    let m = compare_exact(&[-1i64, -1, 5], &[-1, 5, 5]).expect_err("sentinel");
    assert_eq!(m.index, 1);
    let m = compare_exact(&[Vec::<u8>::new()], &[vec![0u8]]).expect_err("empty vs listed");
    assert_eq!(m.index, 0);
    // A length mismatch names both lengths at the shorter length.
    let m = compare_exact(&[1u8, 2, 3], &[1, 2]).expect_err("length");
    assert_eq!(m.index, 2);
    assert_eq!(m.observed, Some(1.0));
    assert!(m.rust.contains("3 elements") && m.oracle.contains("2 elements"));
    // An exact mismatch on a non-numeric type has no observed number: `None`, not NaN (KI-P1).
    let m = compare_exact(&["QB"], &["RB"]).expect_err("categorical");
    assert_eq!((m.observed, m.limit), (None, Some(0.0)));
}

#[test]
fn class_a_passes_and_fails_with_the_index() {
    let oracle = [0.1, 0.2, f64::NAN, -3.0];
    // 8e-13 rather than 1e-12: the sum rounds at ulp(3.0) ≈ 4.4e-16, so a difference of
    // exactly 1e-12 can land just above the bound.
    let rust = [0.1 + 5e-13, 0.2, f64::NAN, -3.0 - 8e-13];
    let report = compare_class_a(&rust, &oracle).expect("within 1e-12");
    assert_eq!(report.n, 4);
    assert!(report.max_abs_error <= CLASS_A_ABS_TOL);
    let bad = [0.1, 0.2, f64::NAN, -3.0 + 3e-12];
    let m: Mismatch = compare_class_a(&bad, &oracle).expect_err("3e-12 > 1e-12");
    assert_eq!(m.index, 3);
    assert_eq!(m.limit, Some(CLASS_A_ABS_TOL));
    assert!(m.observed.is_some_and(|d| d > CLASS_A_ABS_TOL));
    assert!(m.rust.contains("-2.99") || m.rust.contains("-3.0"));
    assert_eq!(m.oracle, "-3.0");
    // NaN positions must coincide; the mismatch carries no observed number.
    let m = compare_class_a(&[0.1, 0.2, 0.3, -3.0], &oracle).expect_err("NaN position");
    assert_eq!(m.index, 2);
    assert_eq!(m.observed, None);
    assert_eq!(m.oracle, "NaN");
}

#[test]
fn class_a_prime_passes_and_fails_with_the_index() {
    let oracle = [4.0, -8.0, 2.0, 0.0];
    let rust = [4.0, -8.0 + 4e-9, 2.0, 0.0]; // 4e-9 / 8 = 5e-10 ≤ 1e-9
    let report = compare_class_a_prime(&rust, &oracle).expect("5e-10 relative");
    assert_eq!(report.n, 4);
    assert!(report.max_rel_error <= CLASS_A_PRIME_REL_TOL);
    let bad = [4.0 + 1e-8, -8.0, 2.0, 0.0]; // 1e-8 / 8 = 1.25e-9 > 1e-9
    let m = compare_class_a_prime(&bad, &oracle).expect_err("1.25e-9 relative");
    assert_eq!(m.index, 0);
    assert_eq!(m.limit, Some(CLASS_A_PRIME_REL_TOL));
    assert!(m.observed.is_some_and(|d| d > CLASS_A_PRIME_REL_TOL));
    // The degenerate case ‖p‖∞ = 0 requires ‖r‖∞ = 0.
    assert!(compare_class_a_prime(&[0.0, -0.0], &[0.0, 0.0]).is_ok());
    let m = compare_class_a_prime(&[0.0, 1e-200], &[0.0, 0.0]).expect_err("degenerate");
    assert_eq!(m.index, 1);
    assert_eq!(m.limit, Some(0.0));
    // A length mismatch is a mismatch, not a panic.
    assert!(compare_class_a_prime(&[1.0], &[1.0, 1.0]).is_err());
}

#[test]
fn class_b_checks_all_three_parts() {
    let dense = [10.0, -5.0, 0.25, 1e-3];
    let cg = [10.0 + 5e-9, -5.0, 0.25, 1e-3]; // 5e-9 / 10 = 5e-10 ≤ 1e-9
    let ok = ClassBInputs {
        converged: true,
        relative_residual: 4e-7,
        tol_cg: 1e-6,
        beta_cg: &cg,
        beta_dense: &dense,
    };
    let report = compare_class_b(&ok).expect("converged, residual ≤ tol_cg, dense agreement");
    assert_eq!(report.n, 4);
    assert!(report.max_rel_error <= CLASS_B_DENSE_REL_TOL);

    let m = compare_class_b(&ClassBInputs {
        converged: false,
        ..ok.clone()
    })
    .expect_err("part 1");
    assert_eq!(m.index, 0);
    assert_eq!(m.rust, "converged = false");
    // The convergence part is boolean: no observed number, no bound (never 0 / 1 stand-ins).
    assert_eq!((m.observed, m.limit), (None, None));

    let m = compare_class_b(&ClassBInputs {
        relative_residual: 2e-6,
        ..ok.clone()
    })
    .expect_err("part 2");
    assert_eq!((m.index, m.observed, m.limit), (0, Some(2e-6), Some(1e-6)));

    let far = [10.0, -5.0 + 2e-8, 0.25, 1e-3]; // 2e-8 / 10 = 2e-9 > 1e-9
    let m = compare_class_b(&ClassBInputs {
        beta_cg: &far,
        ..ok
    })
    .expect_err("part 3");
    assert_eq!(m.index, 1);
    assert_eq!(m.limit, Some(CLASS_B_DENSE_REL_TOL));
}

#[test]
fn bitwise_distinguishes_nan_payloads() {
    let a = f64::from_bits(0x7ff8_0000_0000_0000);
    let b = f64::from_bits(0x7ff8_0000_0000_0001);
    assert!(compare_bitwise(&[a, 1.0], &[a, 1.0]).is_ok());
    let m = compare_bitwise(&[a, 1.0], &[b, 1.0]).expect_err("payload");
    assert_eq!(m.index, 0);
    assert!(m.rust.contains("0x7ff8000000000000"));
    assert!(m.oracle.contains("0x7ff8000000000001"));
    assert_eq!(m.observed, None, "no |r − p| exists between NaNs");
    // The value-exact check (§6 exact class for floats; S-1) ignores payload and sign of zero.
    assert!(compare_exact_f64(&[a, 0.0], &[b, -0.0]).is_ok());
    let m = compare_exact_f64(&[a, 0.0], &[a, 2.0]).expect_err("value");
    assert_eq!((m.index, m.observed, m.limit), (1, Some(2.0), Some(0.0)));
    let m = compare_exact_f64(&[1.0, 0.0], &[a, 0.0]).expect_err("NaN position");
    assert_eq!((m.index, m.observed), (0, None));
}
