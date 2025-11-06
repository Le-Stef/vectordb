#[inline(always)]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();

    // optimisation pour petits vecteurs
    if len < 8 {
        let mut sum = 0.0;
        for i in 0..len {
            sum += a[i] * b[i];
        }
        return sum;
    }

    // déroulement de boucle pour vectorisation auto
    let chunks = len / 4;
    let remainder = len % 4;

    let mut sum1 = 0.0;
    let mut sum2 = 0.0;
    let mut sum3 = 0.0;
    let mut sum4 = 0.0;

    let mut i = 0;
    for _ in 0..chunks {
        sum1 += a[i] * b[i];
        sum2 += a[i + 1] * b[i + 1];
        sum3 += a[i + 2] * b[i + 2];
        sum4 += a[i + 3] * b[i + 3];
        i += 4;
    }

    let mut sum = sum1 + sum2 + sum3 + sum4;

    // reste
    for j in 0..remainder {
        sum += a[i + j] * b[i + j];
    }

    sum
}

#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - dot_product(a, b)
}

#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    dot_product(a, b)
}

pub fn normalize_l2(vector: &mut [f32]) {
    let sq_sum: f32 = vector.iter().map(|x| x * x).sum();
    let norm = sq_sum.sqrt();

    if norm > 1e-10 {  // éviter division par zéro
        let inv_norm = 1.0 / norm;
        for v in vector.iter_mut() {
            *v *= inv_norm;
        }
    }
}

pub fn normalized_l2(vector: &[f32]) -> Vec<f32> {
    let mut result = vector.to_vec();
    normalize_l2(&mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = dot_product(&a, &b);
        assert_eq!(result, 32.0); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn test_normalize_l2() {
        let mut v = vec![3.0, 4.0];
        normalize_l2(&mut v);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_distance_normalized() {
        let mut a = vec![1.0, 0.0];
        let mut b = vec![0.0, 1.0];
        normalize_l2(&mut a);
        normalize_l2(&mut b);

        let dist = cosine_distance(&a, &b);
        assert!((dist - 1.0).abs() < 1e-6); // Vecteurs orthogonaux
    }
}
