use nalgebra::{stack, DMatrix, DVector};

pub struct ContinuousTransferFunction {
    num: DVector<f64>,
    den: DVector<f64>,
}

impl ContinuousTransferFunction {
    pub fn new(num: DVector<f64>, den: DVector<f64>) -> Self {
        Self { num, den }
    }
}

pub struct DiscreteTransferFunction {
    num: DVector<f64>,
    den: DVector<f64>,
    inputs: DVector<f64>,
    outputs: DVector<f64>,
    #[allow(unused)]
    dt: f64,
}

impl DiscreteTransferFunction {
    pub fn new(num: DVector<f64>, den: DVector<f64>, dt: f64) -> Self {
        let inputs = DVector::zeros(num.len());
        let outputs = DVector::zeros(den.len());

        Self {
            num,
            den,
            inputs,
            outputs,
            dt,
        }
    }

    pub fn step(&mut self, input: f64) -> f64 {
        let mut output = 0.0;

        for i in (1..self.inputs.len()).rev() {
            self.inputs[i] = self.inputs[i - 1];
        }
        self.inputs[0] = input;
        output += self.num.dot(&self.inputs);

        for i in (1..self.outputs.len()).rev() {
            self.outputs[i] = self.outputs[i - 1];
        }
        output -= self
            .den
            .rows(1, self.den.len() - 1)
            .dot(&self.outputs.rows(1, self.outputs.len() - 1));
        output /= self.den[0];
        self.outputs[0] = output;

        output
    }
}

pub struct ContinuousStateSpace {
    pub a: DMatrix<f64>,
    pub b: DMatrix<f64>,
    pub c: DMatrix<f64>,
    pub d: DMatrix<f64>,
}

impl ContinuousStateSpace {
    pub fn new(a: DMatrix<f64>, b: DMatrix<f64>, c: DMatrix<f64>, d: DMatrix<f64>) -> Self {
        Self { a, b, c, d }
    }

    pub fn to_discrete(&self, dt: f64, alpha: f64) -> DiscreteStateSpace {
        let a = self.a.clone();
        let b = self.b.clone();
        let c = self.c.clone();
        let d = self.d.clone();

        let ima = DMatrix::identity(a.nrows(), a.nrows()) - alpha * dt * &a;
        let ima_lu = ima.clone().lu();
        let ad = ima_lu
            .solve(&(DMatrix::identity(a.nrows(), a.nrows()) + (1.0 - alpha) * dt * &a))
            .unwrap();
        let bd = ima_lu.solve(&(dt * &b)).unwrap();
        let cd = ima
            .transpose()
            .lu()
            .solve(&c.transpose())
            .unwrap()
            .transpose();
        let dd = d + alpha * (&c * &bd);

        DiscreteStateSpace::new(ad, bd, cd, dd, dt)
    }
}

impl From<ContinuousTransferFunction> for ContinuousStateSpace {
    fn from(tf: ContinuousTransferFunction) -> Self {
        assert!(
            tf.den.len() >= tf.num.len(),
            "The order of the denominator must be greater than or equal to the order of the numerator."
        );

        let n = tf.den.len() - 1; // Order

        // Normalize the numerator and denominator
        let num = stack![DVector::zeros(tf.den.len() - tf.num.len()); tf.num.clone()] / tf.den[0];
        let den = tf.den.clone() / tf.den[0];

        let a = stack![
            -den.rows(1, n).transpose();
            DMatrix::identity(n - 1, n)
        ];
        let b = DMatrix::identity(n, 1);
        let c = DMatrix::from_row_slice(1, n, num.rows(1, n).as_slice())
            - num[0] * DMatrix::from_row_slice(1, n, &den.rows(1, n).as_slice());
        let d = DMatrix::from_row_slice(1, 1, &[num[0]]);
        ContinuousStateSpace { a, b, c, d }
    }
}

pub struct DiscreteStateSpace {
    pub a: DMatrix<f64>,
    pub b: DMatrix<f64>,
    pub c: DMatrix<f64>,
    pub d: DMatrix<f64>,
    pub x: DVector<f64>,
    pub dt: f64,
}

impl DiscreteStateSpace {
    pub fn new(
        a: DMatrix<f64>,
        b: DMatrix<f64>,
        c: DMatrix<f64>,
        d: DMatrix<f64>,
        dt: f64,
    ) -> Self {
        let x = DVector::zeros(a.nrows());
        Self { a, b, c, d, x, dt }
    }

    pub fn step(&mut self, input: f64) -> f64 {
        let output = &self.c * &self.x + &self.d * input;
        self.x = &self.a * &self.x + &self.b * input;

        output[0]
    }
}

fn characteristic_polynomial(matrix: &DMatrix<f64>) -> DVector<f64> {
    assert_eq!(matrix.nrows(), matrix.ncols(), "Matrix must be square.");

    // nalgebra で固有値を計算
    // （本来は複素数が返るが、すべて実数と仮定する）
    let eigenvalues = matrix.clone().eigenvalues().unwrap();

    // (λ - λ_i) の積を 展開して係数を求める
    let mut coeffs = DVector::from_vec(vec![1.0]);

    for root in eigenvalues.iter() {
        let degree = coeffs.len();
        let mut new_coeffs = DVector::zeros(degree + 1);
        for i in 0..degree {
            // x^i の係数に x を掛けると x^(i+1) の係数になる
            new_coeffs[i] += coeffs[i];
            // x^i の係数に (-root) を掛けると、-root * x^i になる
            new_coeffs[i + 1] -= root * coeffs[i];
        }
        coeffs = new_coeffs;
    }

    coeffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_step_discrete_transfer_function() {
        let num = DVector::from_vec(vec![1.3]);
        let den = DVector::from_vec(vec![2.0, 1.5]);
        let dt = 0.1;
        let mut discrete_tf = DiscreteTransferFunction::new(num, den, dt);

        let inputs = vec![0.2, 0.4, 0.6, 0.8, 1.0];
        let outputs = inputs
            .iter()
            .map(|input| discrete_tf.step(*input))
            .collect::<Vec<_>>();
        let expected_outputs = vec![0.13, 0.1625, 0.268125, 0.31890625, 0.4108203125];

        for (output, expected_output) in outputs.iter().zip(expected_outputs.iter()) {
            assert_relative_eq!(output, expected_output);
        }
    }

    #[test]
    fn test_step_discrete_state_space() {
        let a = DMatrix::from_row_slice(2, 2, &[-2.0, -3.0, 1.0, 0.0]);
        let b = DMatrix::from_row_slice(2, 1, &[1.0, 0.0]);
        let c = DMatrix::from_row_slice(1, 2, &[1.0, 2.0]);
        let d = DMatrix::from_row_slice(1, 1, &[2.0]);
        let dt = 0.1;
        let mut discrete_state_space = DiscreteStateSpace::new(a, b, c, d, dt);

        let inputs = vec![0.2, 0.4, 0.6, 0.8, 1.0];
        let outputs = inputs
            .iter()
            .map(|input| discrete_state_space.step(*input))
            .collect::<Vec<_>>();
        let expected_outputs = vec![0.4, 1.0, 1.6, 1.6, 2.8];

        for (output, expected_output) in outputs.iter().zip(expected_outputs.iter()) {
            assert_relative_eq!(output, expected_output);
        }
    }

    #[test]
    fn test_continuous_transfer_function_to_continuous_state_space() {
        let num = DVector::from_vec(vec![1.0, 3.0, 3.0]);
        let den = DVector::from_vec(vec![1.0, 2.0, 1.0]);

        let tf = ContinuousTransferFunction::new(num, den);
        let ss = ContinuousStateSpace::from(tf);

        let expected_a = DMatrix::from_row_slice(2, 2, &[-2.0, -1.0, 1.0, 0.0]);
        let expected_b = DMatrix::from_row_slice(2, 1, &[1.0, 0.0]);
        let expected_c = DMatrix::from_row_slice(1, 2, &[1.0, 2.0]);
        let expected_d = DMatrix::from_row_slice(1, 1, &[1.0]);

        assert_relative_eq!(ss.a, expected_a);
        assert_relative_eq!(ss.b, expected_b);
        assert_relative_eq!(ss.c, expected_c);
        assert_relative_eq!(ss.d, expected_d);
    }

    #[test]
    fn test_continuous_transfer_function_to_continuous_state_space_different_order() {
        let num = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let den = DVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        let tf = ContinuousTransferFunction::new(num, den);
        let ss = ContinuousStateSpace::from(tf);

        let expected_a =
            DMatrix::from_row_slice(3, 3, &[-2.0, -3.0, -4.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let expected_b = DMatrix::from_row_slice(3, 1, &[1.0, 0.0, 0.0]);
        let expected_c = DMatrix::from_row_slice(1, 3, &[1.0, 2.0, 3.0]);
        let expected_d = DMatrix::from_row_slice(1, 1, &[0.0]);

        assert_relative_eq!(ss.a, expected_a);
        assert_relative_eq!(ss.b, expected_b);
        assert_relative_eq!(ss.c, expected_c);
        assert_relative_eq!(ss.d, expected_d);
    }

    #[test]
    fn test_continuous_state_space_to_discrete_state_space() {
        let ac = DMatrix::identity(2, 2);
        let bc = DMatrix::from_row_slice(2, 1, &[0.5, 0.5]);
        let cc = DMatrix::from_row_slice(3, 2, &[0.75, 1.0, 1.0, 1.0, 1.0, 0.25]);
        let dc = DMatrix::from_row_slice(3, 1, &[0.0, 0.0, -0.33]);
        let continuous_state_space = ContinuousStateSpace::new(ac, bc, cc, dc);

        let dt = 0.5;
        let alpha = 1.0 / 3.0;
        let discrete_state_space = continuous_state_space.to_discrete(dt, alpha);

        let expected_a = 1.6 * DMatrix::identity(2, 2);
        let expected_b = DMatrix::from_row_slice(2, 1, &[0.3, 0.3]);
        let expected_c = DMatrix::from_row_slice(3, 2, &[0.9, 1.2, 1.2, 1.2, 1.2, 0.3]);
        let expected_d = DMatrix::from_row_slice(3, 1, &[0.175, 0.2, -0.205]);

        assert_relative_eq!(discrete_state_space.a, expected_a);
        assert_relative_eq!(discrete_state_space.b, expected_b);
        assert_relative_eq!(discrete_state_space.c, expected_c);
        assert_relative_eq!(discrete_state_space.d, expected_d);
    }
}
