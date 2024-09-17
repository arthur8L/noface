use cudarc::driver::CudaDevice;

use crate::{Error, Result};

use super::{ModelData, RecgnData, TensorData};

// tar: (n, 3, 128, 128) | src: (1, 512)
pub struct SwapModel(pub ort::Session);

impl SwapModel {
    // inswapper_128.onnx
    pub fn new(onnx_path: std::path::PathBuf) -> Result<Self> {
        Ok(Self(super::start_session_from_file(onnx_path)?))
    }

    pub fn run(
        &self,
        tar: TensorData,
        src: RecgnData,
        cuda_device: Option<&std::sync::Arc<CudaDevice>>,
    ) -> Result<TensorData> {
        if let Some(cuda) = cuda_device {
            self.run_with_cuda(tar, src, cuda)
        } else {
            self.run_with_cpu(tar, src)
        }
    }

    fn run_with_cpu(&self, tar: TensorData, src: RecgnData) -> Result<TensorData> {
        let dim = tar.dim();

        let outputs = self
            .0
            .run(ort::inputs![tar.0, src.0].map_err(Error::ModelError)?)
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
        tar: TensorData,
        src: impl ModelData,
        cuda: &std::sync::Arc<CudaDevice>,
    ) -> Result<TensorData> {
        let tar_dim = tar.dim();

        let (tar_tensor, src_tensor) = (tar.to_tensor_ref(cuda)?, src.to_tensor_ref(cuda)?);

        let outputs = self
            .0
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
