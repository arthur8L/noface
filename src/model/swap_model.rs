use cudarc::driver::CudaDevice;

use crate::{Error, Result};

use super::{EmbedData, ModelData, TensorData};

type SrcArray = ndarray::Array<f32, ndarray::Dim<[usize; 2]>>;

// tar: (n, 3, 128, 128) | src: (1, 512)
pub struct SwapModel(pub ort::Session);

impl SwapModel {
    // inswapper_128.onnx
    pub fn new(onnx_path: std::path::PathBuf) -> Result<Self> {
        Ok(SwapModel(super::start_session_from_file(onnx_path)?))
    }

    pub fn process(&self, tar: TensorData, src: SrcArray) -> Result<TensorData> {
        let dim = tar.dim();

        let outputs = self
            .run(ort::inputs![tar.0, src].map_err(Error::ModelError)?)
            .map_err(Error::ModelError)?;

        Ok(outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(Error::ModelError)?
            .to_shape(dim)
            .map_err(Error::as_unknown_error)?
            .into_owned()
            .into())
    }

    pub fn process_with_cuda(
        &self,
        cuda: &std::sync::Arc<CudaDevice>,
        tar: TensorData,
        src: impl ModelData,
    ) -> Result<TensorData> {
        let tar_dim = tar.dim();

        let (tar_tensor, src_tensor) = (tar.to_tensor_ref(cuda)?, src.to_tensor_ref(cuda)?);

        let outputs = self
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

impl std::ops::Deref for SwapModel {
    type Target = ort::Session;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SwapModel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}