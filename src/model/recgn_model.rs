use crate::{Error, Result};

use super::{ModelData, RecgnData, TensorData};

pub struct RecgnModel(pub ort::Session);

impl RecgnModel {
    // w600k_r50.onnx
    pub fn new(onnx_path: std::path::PathBuf) -> Result<Self> {
        Ok(Self(super::start_session_from_file(onnx_path)?))
    }

    // (n, 3, 112, 112)
    pub fn run(
        &self,
        data: TensorData,
        cuda_device: Option<&std::sync::Arc<cudarc::driver::CudaDevice>>,
    ) -> Result<RecgnData> {
        if let Some(cuda) = cuda_device {
            self.run_with_cuda(data, cuda)
        } else {
            self.run_with_cpu(data)
        }
    }
    pub fn run_with_cpu(&self, data: TensorData) -> Result<RecgnData> {
        let outputs = self
            .0
            .run(ort::inputs![data.0].map_err(Error::ModelError)?)
            .map_err(Error::ModelError)?;

        Ok(outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(Error::ModelError)?
            .to_shape((1, 512))
            .map_err(Error::as_unknown_error)?
            .to_owned()
            .into())
    }

    pub fn run_with_cuda(
        &self,
        data: TensorData,
        cuda: &std::sync::Arc<cudarc::driver::CudaDevice>,
    ) -> Result<RecgnData> {
        let tensor = data.to_tensor_ref(cuda)?;
        let outputs = self.0.run([tensor.into()]).map_err(Error::ModelError)?;
        Ok(outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(Error::ModelError)?
            .to_shape((1, 512))
            .map_err(Error::as_unknown_error)?
            .into_owned()
            .into())
    }
}
