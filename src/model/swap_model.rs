use cudarc::driver::CudaDevice;

use crate::{Error, Result};

use super::{
    data::{get_tensor_ref, graph::InitialGraphOutput, VectorizedTensor},
    InputSizeMatrix, Tensor,
};

//https://github.com/deepinsight/insightface/blob/master/python-package/insightface/model_zoo/inswapper.py
// tar: (n, 3, 128, 128) | src: (1, 512)
pub struct SwapModel {
    input_size: (usize, usize),
    input_size_mat: InputSizeMatrix,
    session: ort::Session,
    pub graph: InitialGraphOutput,
}

impl SwapModel {
    // inswapper_128.onnx
    #[tracing::instrument(name = "Initialize swap model", err)]
    pub fn new(onnx_path: std::path::PathBuf) -> Result<Self> {
        Ok(Self {
            input_size: (128, 128),
            input_size_mat: InputSizeMatrix::from_shape_fn((1, 3, 128, 128), |d| d),
            session: super::start_session_from_file(onnx_path)?,
            graph: InitialGraphOutput::get()?,
        })
    }

    pub fn run(
        &mut self,
        mut tar: Tensor,
        src: VectorizedTensor,
        cuda_device: Option<&std::sync::Arc<CudaDevice>>,
    ) -> Result<Tensor> {
        // (n, c, h, w)
        let (_, _, dy, dx) = tar.dim();
        if dy != self.input_size.1 && dx != self.input_size.0 {
            tar = tar.resize_with_matrix(&mut self.input_size_mat);
        }
        tar.to_normalization(super::data::Normal::ZeroToP1);
        let result = {
            if let Some(cuda) = cuda_device {
                self.run_with_cuda(tar, src, cuda)
            } else {
                self.run_with_cpu(tar, src)
            }
        }?;

        Ok(result)
    }

    fn run_with_cpu(&self, tar: Tensor, src: VectorizedTensor) -> Result<Tensor> {
        let dim = tar.dim();

        let outputs = self
            .session
            .run(ort::inputs![tar.data, src.0].map_err(Error::ModelError)?)
            .map_err(Error::ModelError)?;

        Ok(outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(Error::ModelError)?
            .to_shape(dim)
            .map_err(Error::as_unknown_error)?
            .into_owned()
            .into())
    }

    fn run_with_cuda(
        &self,
        tar: Tensor,
        src: VectorizedTensor,
        cuda: &std::sync::Arc<CudaDevice>,
    ) -> Result<Tensor> {
        let (tar_dim, src_dim) = (tar.dim(), src.dim());

        let (tar_dd, src_dd) = rayon::join(
            || tar.to_cuda_slice(cuda),
            || {
                cuda.htod_sync_copy(&src.0.into_raw_vec_and_offset().0)
                    .map_err(crate::Error::CudaError)
            },
        );

        let (tar_tensor, src_tensor) = (
            get_tensor_ref(
                &tar_dd?,
                vec![
                    tar_dim.0 as i64,
                    tar_dim.1 as i64,
                    tar_dim.2 as i64,
                    tar_dim.3 as i64,
                ],
            )?,
            get_tensor_ref(&src_dd?, vec![src_dim.0 as i64, src_dim.1 as i64])?,
        );

        let outputs = self
            .session
            .run([tar_tensor.into(), src_tensor.into()])
            .map_err(Error::ModelError)?;

        Ok(outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(Error::ModelError)?
            .to_shape(tar_dim)
            .map_err(Error::as_unknown_error)?
            .into_owned()
            .into())
    }
}
