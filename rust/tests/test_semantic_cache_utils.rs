use coordination_patterns::semantic_cache::cosine_similarity;

#[test]
fn test_identical_vectors() {
    let a = [1.0, 2.0, 3.0];
    assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-10);
}

#[test]
fn test_orthogonal_vectors() {
    let a = [1.0, 0.0];
    let b = [0.0, 1.0];
    assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-10);
}

#[test]
fn test_opposite_vectors() {
    let a = [1.0, 1.0];
    let b = [-1.0, -1.0];
    assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 1e-10);
}

#[test]
fn test_zero_vector() {
    let a = [0.0, 0.0];
    let b = [1.0, 2.0];
    assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-10);
}

#[test]
fn test_different_magnitude_same_direction() {
    let a = [1.0, 1.0];
    let b = [2.0, 2.0];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);
}
