"""Tests for semantic cache utilities."""

import pytest

from coordination_patterns.semantic_cache.utils import cosine_similarity


def test_identical_vectors():
    a = [1.0, 2.0, 3.0]
    assert cosine_similarity(a, a) == pytest.approx(1.0)


def test_orthogonal_vectors():
    a = [1.0, 0.0]
    b = [0.0, 1.0]
    assert cosine_similarity(a, b) == 0.0


def test_opposite_vectors():
    a = [1.0, 1.0]
    b = [-1.0, -1.0]
    assert cosine_similarity(a, b) == pytest.approx(-1.0)


def test_zero_vector():
    a = [0.0, 0.0]
    b = [1.0, 2.0]
    assert cosine_similarity(a, b) == 0.0


def test_different_magnitude_same_direction():
    a = [1.0, 1.0]
    b = [2.0, 2.0]
    assert cosine_similarity(a, b) == pytest.approx(1.0)
