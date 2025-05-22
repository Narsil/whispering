use super::N_SAMPLES;
use ndarray::{Array, Array2, ArrayBase, ArrayD, Dim, IxDynImpl, OwnedRepr};

#[cfg(not(any(feature = "cuda", feature = "metal")))]
use ort::execution_providers::CPUExecutionProvider;
#[cfg(feature = "cuda")]
use ort::execution_providers::CUDAExecutionProvider;
#[cfg(feature = "metal")]
use ort::execution_providers::CoreMLExecutionProvider;

use ort::session::{Session, SessionInputs};
use std::path::Path;

#[derive(Debug)]
pub struct Silero {
    session: Session,
    sample_rate: ArrayBase<OwnedRepr<i64>, Dim<[usize; 1]>>,
    frame: ArrayBase<OwnedRepr<f32>, Dim<[usize; 2]>>,
    state: ArrayBase<OwnedRepr<f32>, Dim<IxDynImpl>>,
}

impl Silero {
    pub fn new(sample_rate: i64, model_path: impl AsRef<Path>) -> Result<Self, ort::Error> {
        #[cfg(feature = "cuda")]
        let provider = CUDAExecutionProvider::default().build().error_on_failure();
        #[cfg(feature = "metal")]
        let provider = CoreMLExecutionProvider::default()
            .build()
            .error_on_failure();
        #[cfg(not(any(feature = "cuda", feature = "metal")))]
        let provider = CPUExecutionProvider::default().build().error_on_failure();
        let session = Session::builder()?
            .with_execution_providers([provider])?
            .commit_from_file(model_path)?;
        let state = ArrayD::<f32>::zeros([2, 1, 128].as_slice());
        let sample_rate = Array::from_shape_vec([1], vec![sample_rate]).unwrap();
        let frame = Array2::<f32>::zeros([1, N_SAMPLES]);
        Ok(Self {
            frame,
            session,
            sample_rate,
            state,
        })
    }

    pub fn calc_level(&mut self, audio_frame: &[f32; N_SAMPLES]) -> Result<f32, ort::Error> {
        self.frame.iter_mut().zip(audio_frame).for_each(|(s, ns)| {
            *s = *ns;
        });
        let inps = ort::inputs![
            self.frame.clone(),
            std::mem::take(&mut self.state),
            self.sample_rate.clone(),
        ]?;
        let res = self.session.run(SessionInputs::ValueSlice::<3>(&inps))?;
        self.state = res["stateN"].try_extract_tensor().unwrap().to_owned();
        let output = *res["output"]
            .try_extract_raw_tensor::<f32>()
            .unwrap()
            .1
            .first()
            .unwrap();
        Ok(output)
    }
}
