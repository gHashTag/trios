pub struct BigramHashTable {
    pub vocab_size: usize,
    pub dim: usize,
    pub embeddings: Vec<Vec<f32>>,
}

impl BigramHashTable {
    pub fn new(vocab_size: usize, dim: usize) -> Self {
        let mut embeddings = Vec::with_capacity(vocab_size);
        for _ in 0..vocab_size {
            let mut row = Vec::with_capacity(dim);
            for d in 0..dim {
                let v = ((d as f64 * 0.618033988749895).fract() - 0.5) * 0.1;
                row.push(v as f32);
            }
            embeddings.push(row);
        }
        Self {
            vocab_size,
            dim,
            embeddings,
        }
    }

    pub fn embed(&self, token_id: usize) -> &[f32] {
        &self.embeddings[token_id % self.vocab_size]
    }

    pub fn embed_bigram(&self, t1: usize, t2: usize) -> Vec<f32> {
        let e1 = self.embed(t1);
        let e2 = self.embed(t2);
        e1.iter().zip(e2.iter()).map(|(a, b)| a + b).collect()
    }

    pub fn lookup_f32(&self, token_id: usize, out: &mut [f32]) {
        let emb = self.embed(token_id);
        let len = out.len().min(emb.len());
        out[..len].copy_from_slice(&emb[..len]);
    }

    pub fn gradient_step(&mut self, token_id: usize, grad: &[f32], lr: f32) {
        let emb = &mut self.embeddings[token_id % self.vocab_size];
        for (i, g) in grad.iter().enumerate().take(self.dim) {
            if i < emb.len() {
                emb[i] -= lr * g;
            }
        }
    }

    pub fn param_count(&self) -> usize {
        self.vocab_size * self.dim
    }
}

pub struct SmearGate {
    pub dim: usize,
    pub weights: Vec<f32>,
    pub bias: f32,
}

impl SmearGate {
    pub fn new(dim: usize) -> Self {
        let inv_phi = 1.0 / 1.618033988749895;
        let weights = vec![inv_phi as f32; dim];
        Self {
            dim,
            weights,
            bias: 0.0,
        }
    }

    pub fn forward(&self, embedding: &[f32], context_mean: &[f32]) -> Vec<f32> {
        let gate = self.compute_gate(embedding, context_mean);
        embedding
            .iter()
            .zip(context_mean.iter())
            .zip(gate.iter())
            .map(|((e, c), g)| e * (1.0 - g) + c * g)
            .collect()
    }

    fn compute_gate(&self, embedding: &[f32], context: &[f32]) -> Vec<f32> {
        let dot: f32 = embedding
            .iter()
            .zip(context.iter())
            .zip(self.weights.iter())
            .map(|((e, c), w)| (e - c) * w)
            .sum();
        let sigmoid = 1.0 / (1.0 + (-dot - self.bias).exp());
        vec![sigmoid; self.dim]
    }

    pub fn gradient_step(&mut self, grad_w: &[f32], grad_b: f32, lr: f32) {
        for (i, w) in self.weights.iter_mut().enumerate() {
            if i < grad_w.len() {
                *w -= lr * grad_w[i];
            }
        }
        self.bias -= lr * grad_b;
    }

    pub fn param_count(&self) -> usize {
        self.dim + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigram_hash_dimensions() {
        let bh = BigramHashTable::new(729, 64);
        assert_eq!(bh.embed(0).len(), 64);
        assert_eq!(bh.vocab_size, 729);
        assert_eq!(bh.param_count(), 729 * 64);
    }

    #[test]
    fn bigram_hash_embedding_shape() {
        let bh = BigramHashTable::new(729, 64);
        let e = bh.embed_bigram(100, 200);
        assert_eq!(e.len(), 64);
    }

    #[test]
    fn smear_gate_shape() {
        let sg = SmearGate::new(64);
        let emb = vec![1.0f32; 64];
        let ctx = vec![0.0f32; 64];
        let out = sg.forward(&emb, &ctx);
        assert_eq!(out.len(), 64);
    }

    #[test]
    fn smear_gate_param_count() {
        let sg = SmearGate::new(64);
        assert_eq!(sg.param_count(), 65);
    }

    #[test]
    fn gradient_step_modifies_weights() {
        let mut bh = BigramHashTable::new(10, 4);
        let before = bh.embed(0)[0];
        let grad = vec![1.0, 0.0, 0.0, 0.0];
        bh.gradient_step(0, &grad, 0.1);
        assert!(bh.embed(0)[0] != before);
    }
}
