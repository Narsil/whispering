#[derive(Clone, Copy)]
pub struct Resample {
    pub samplerate_in: u32,
    pub samplerate_out: u32,
    pub in_channels: u16,
}

pub fn audio_resample(
    data: &[f32],
    sample_rate0: u32,
    sample_rate: u32,
    channels: u16,
) -> Vec<f32> {
    use samplerate::{ConverterType, convert};
    convert(
        sample_rate0 as _,
        sample_rate as _,
        channels as _,
        ConverterType::SincBestQuality,
        data,
    )
    .unwrap_or_default()
}
