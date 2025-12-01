//! Dynamic attention mechanism
//!
//! Implements a simple 2-layer MLP for computing attention scales:
//! hidden = tanh(W1 * x + b1)
//! scale = abs(W2 * hidden + b2) + 0.1
//!
//! Uses deterministic weight initialization based on embedding dimension.

use anyhow::Result;
use std::f32::consts::PI;

/// Compute attention scale using 2-layer MLP
///
/// # Arguments
/// * `context_embedding` - Input embedding vector (e.g., 384-dim)
/// * `embedding_dim` - Expected dimensionality of the embedding
///
/// # Returns
/// Positive attention scale value (always > 0.1)
pub fn compute_attention_scale(context_embedding: &[f32], embedding_dim: usize) -> Result<f32> {
    if context_embedding.is_empty() {
        anyhow::bail!("Context embedding cannot be empty");
    }

    if context_embedding.len() != embedding_dim {
        anyhow::bail!(
            "Embedding dimension mismatch: expected {}, got {}",
            embedding_dim,
            context_embedding.len()
        );
    }

    // Hidden layer size (smaller than input)
    let hidden_size = (embedding_dim / 4).max(8);

    // Initialize weights deterministically based on dimensions
    let w1 = init_weight_matrix(embedding_dim, hidden_size);
    let b1 = init_bias_vector(hidden_size);
    let w2 = init_weight_vector(hidden_size);
    let b2 = init_bias_scalar(hidden_size);

    // Layer 1: hidden = tanh(W1 * x + b1)
    let hidden = matmul_add_bias(&w1, context_embedding, &b1, hidden_size);
    let hidden_activated: Vec<f32> = hidden.iter().map(|&h| h.tanh()).collect();

    // Layer 2: scale = abs(W2 * hidden + b2) + 0.1
    let raw_scale = dot_product(&w2, &hidden_activated) + b2;
    let scale = raw_scale.abs() + 0.1;

    Ok(scale)
}

/// Initialize weight matrix W1 (hidden_size x embedding_dim) deterministically
fn init_weight_matrix(embedding_dim: usize, hidden_size: usize) -> Vec<Vec<f32>> {
    let mut weights = Vec::with_capacity(hidden_size);
    for i in 0..hidden_size {
        let mut row = Vec::with_capacity(embedding_dim);
        for j in 0..embedding_dim {
            // Deterministic initialization using sine function
            let val = ((i * 7 + j * 13) as f32 * PI / embedding_dim as f32).sin() * 0.01;
            row.push(val);
        }
        weights.push(row);
    }
    weights
}

/// Initialize bias vector b1 (hidden_size) deterministically
fn init_bias_vector(hidden_size: usize) -> Vec<f32> {
    (0..hidden_size).map(|i| ((i * 11) as f32 * PI / hidden_size as f32).cos() * 0.01).collect()
}

/// Initialize weight vector W2 (hidden_size) deterministically
fn init_weight_vector(hidden_size: usize) -> Vec<f32> {
    (0..hidden_size).map(|i| ((i * 17) as f32 * PI / hidden_size as f32).sin() * 0.1).collect()
}

/// Initialize bias scalar b2 deterministically
fn init_bias_scalar(hidden_size: usize) -> f32 {
    ((hidden_size * 19) as f32 * PI).cos() * 0.01
}

/// Matrix-vector multiplication with bias addition: W * x + b
fn matmul_add_bias(
    weights: &[Vec<f32>],
    input: &[f32],
    bias: &[f32],
    output_size: usize,
) -> Vec<f32> {
    let mut output = Vec::with_capacity(output_size);
    for i in 0..output_size {
        let mut sum = bias[i];
        for (j, &x_j) in input.iter().enumerate() {
            sum += weights[i][j] * x_j;
        }
        output.push(sum);
    }
    output
}

/// Dot product of two vectors
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_weights() {
        let w1_a = init_weight_matrix(384, 96);
        let w1_b = init_weight_matrix(384, 96);
        assert_eq!(w1_a.len(), w1_b.len());
        assert_eq!(w1_a[0].len(), w1_b[0].len());
        for i in 0..w1_a.len() {
            for j in 0..w1_a[i].len() {
                assert_eq!(w1_a[i][j], w1_b[i][j]);
            }
        }
    }

    #[test]
    fn test_matrix_vector_multiply() {
        let weights = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let input = vec![0.5, 0.3];
        let bias = vec![0.0, 0.0];
        let result = matmul_add_bias(&weights, &input, &bias, 2);
        assert!((result[0] - 1.1).abs() < 1e-5); // 1.0*0.5 + 2.0*0.3 = 1.1
        assert!((result[1] - 2.7).abs() < 1e-5); // 3.0*0.5 + 4.0*0.3 = 2.7
    }

    #[test]
    fn test_dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = dot_product(&a, &b);
        assert!((result - 32.0).abs() < 1e-5); // 1*4 + 2*5 + 3*6 = 32
    }
}
