use nalgebra::Matrix3;

const KEY_POINTS_LEN: usize = 5;
const ARC_FACE_DST: KeyPoints = KeyPoints([
    (38.2946, 51.6963),
    (73.5318, 51.5014),
    (56.0252, 71.7366),
    (41.5493, 92.3655),
    (70.7299, 92.2041),
]);

#[derive(Debug, Clone)]
pub struct KeyPoints(pub [(f32, f32); KEY_POINTS_LEN]);

impl KeyPoints {
    fn mean(&self) -> (f32, f32) {
        self.0.iter().fold((0., 0.), |accu, p| {
            (
                accu.0 + p.0 / KEY_POINTS_LEN as f32,
                accu.1 + p.1 / KEY_POINTS_LEN as f32,
            )
        })
    }

    #[allow(dead_code)]
    fn variance(&self, mean: Option<(f32, f32)>) -> (f32, f32) {
        let (x_mean, y_mean) = mean.unwrap_or(self.mean());
        let sum = self.0.iter().fold((0., 0.), |accu, p| {
            (
                accu.0 + (p.0 - x_mean).abs().powi(2),
                accu.1 + (p.1 - y_mean).abs().powi(2),
            )
        });
        (sum.0 / KEY_POINTS_LEN as f32, sum.1 / KEY_POINTS_LEN as f32)
    }

    #[allow(dead_code)]
    fn covariance(
        &self,
        oth: &Self,
        src_mean: Option<(f32, f32)>,
        oth_mean: Option<(f32, f32)>,
    ) -> f32 {
        let (src_x_mean, src_y_mean) = src_mean.unwrap_or(self.mean());
        let (oth_x_mean, oth_y_mean) = oth_mean.unwrap_or(oth.mean());
        self.0
            .iter()
            .zip(oth.0.iter())
            .fold(0., |accu, (src_p, oth_p)| {
                accu + ((src_p.0 - src_x_mean) * (oth_p.0 - oth_x_mean)
                    + (src_p.1 - src_y_mean) * (oth_p.0 - oth_y_mean))
            })
            / KEY_POINTS_LEN as f32
    }

    fn covariance_matrix(&self, oth: &Self) {
        todo!();
    }

    pub fn umeyama(&self, dst: &Self) -> nalgebra::Matrix3<f32> {
        use nalgebra::{ArrayStorage, Matrix, Matrix1x2, Matrix2, Matrix2x1};
        use std::ops::Mul;
        let (src_x_mean, src_y_mean) = self.mean();
        let (dst_x_mean, dst_y_mean) = dst.mean();
        let (src_dmean, dst_dmean) = (
            Matrix::from_array_storage(ArrayStorage(
                self.0.map(|(x, y)| [x - src_x_mean, y - src_y_mean]),
            )),
            Matrix::from_array_storage(ArrayStorage(
                dst.0.map(|(x, y)| [x - dst_x_mean, y - dst_y_mean]),
            )),
        );
        let a = std::ops::Mul::mul(dst_dmean, &src_dmean.transpose()) / 5.;
        let svd = Matrix::svd(a, true, true);
        let determinant = a.determinant();

        let mut d = [1f32; 2];
        if determinant < 0. {
            d[1] = -1.;
        }

        let mut t = Matrix2::<f32>::identity();
        let (s, u, v) = (svd.singular_values, svd.u.unwrap(), svd.v_t.unwrap());

        let rank = a.rank(0.00001f32);
        if rank == 0 {
            panic!("Matrix rank is 0");
        }

        if rank == 1 {
            if u.determinant() * v.determinant() > 0. {
                u.mul_to(&v, &mut t);
            } else {
                let s = d[1];
                d[1] = -1.;
                let dg = Matrix2::<f32>::new(d[0], 0., 0., d[1]);

                let udg = u.mul(&dg);
                udg.mul_to(&v, &mut t);
                d[1] = s;
            }
        } else {
            let dg = Matrix2::<f32>::new(d[0], 0., 0., d[1]);
            let udg = u.mul(&dg);
            udg.mul_to(&v, &mut t);
        }

        let ddd = Matrix1x2::new(d[0], d[1]);
        let d_x_s = ddd.mul(s);

        let (var0, var1) = (
            src_dmean.remove_row(0).variance(),
            src_dmean.remove_row(1).variance(),
        );

        let var_sum = var0 + var1;

        let scale = d_x_s.get((0, 0)).unwrap() / var_sum;

        let (dst_mean, src_mean) = (
            Matrix2x1::<f32>::new(dst_x_mean, dst_y_mean),
            Matrix2x1::<f32>::new(src_x_mean, src_y_mean),
        );
        let t_x_src_mean = t.mul(&src_mean);

        let xxx = scale * t_x_src_mean;
        let yyy = dst_mean - xxx;

        let (m13, m23) = (*yyy.get(0).unwrap(), *yyy.get(1).unwrap());

        let m00x22 = t * scale;

        let (m11, m21, m12, m22) = (m00x22.m11, m00x22.m21, m00x22.m12, m00x22.m22);

        Matrix3::<f32>::new(m11, m12, m13, m21, m22, m23, 0., 0., 1.)
    }

    /// (f32, f32, f32) : (R:Rotation Matrix, c:Scale Factor, t: Translation Vector)
    pub fn umeyama_v2(&self, dst: &Self) -> (f32, f32, f32) {
        let (src_x_mean, src_y_mean) = self.mean();
        let (dst_x_mean, dst_y_mean) = dst.mean();

        let (src_variance, dst_variance) = (
            self.variance(Some((src_x_mean, src_y_mean))),
            dst.variance(Some((dst_x_mean, dst_y_mean))),
        );

        //Singular Value Decomposition
        todo!();
    }

    pub fn umeyama_to_arc(&self) -> Matrix3<f32> {
        self.umeyama(&ARC_FACE_DST)
    }
}