use anyhow::{Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, StreamConfig, SupportedStreamConfig};
use hf_hub::api::tokio::ApiBuilder;
use rdev::{EventType, Key, listen, simulate};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, install_logging_hooks,
};

fn key_from_char(byte: u8) -> Option<(Key, bool)> {
    match byte {
        b'a' => Some((Key::KeyA, false)),
        b'b' => Some((Key::KeyB, false)),
        b'c' => Some((Key::KeyC, false)),
        b'd' => Some((Key::KeyD, false)),
        b'e' => Some((Key::KeyE, false)),
        b'f' => Some((Key::KeyF, false)),
        b'g' => Some((Key::KeyG, false)),
        b'h' => Some((Key::KeyH, false)),
        b'i' => Some((Key::KeyI, false)),
        b'j' => Some((Key::KeyJ, false)),
        b'k' => Some((Key::KeyK, false)),
        b'l' => Some((Key::KeyL, false)),
        b'm' => Some((Key::KeyM, false)),
        b'n' => Some((Key::KeyN, false)),
        b'o' => Some((Key::KeyO, false)),
        b'p' => Some((Key::KeyP, false)),
        b'q' => Some((Key::KeyQ, false)),
        b'r' => Some((Key::KeyR, false)),
        b's' => Some((Key::KeyS, false)),
        b't' => Some((Key::KeyT, false)),
        b'u' => Some((Key::KeyU, false)),
        b'v' => Some((Key::KeyV, false)),
        b'w' => Some((Key::KeyW, false)),
        b'x' => Some((Key::KeyX, false)),
        b'y' => Some((Key::KeyY, false)),
        b'z' => Some((Key::KeyZ, false)),

        b'A' => Some((Key::KeyA, true)),
        b'B' => Some((Key::KeyB, true)),
        b'C' => Some((Key::KeyC, true)),
        b'D' => Some((Key::KeyD, true)),
        b'E' => Some((Key::KeyE, true)),
        b'F' => Some((Key::KeyF, true)),
        b'G' => Some((Key::KeyG, true)),
        b'H' => Some((Key::KeyH, true)),
        b'I' => Some((Key::KeyI, true)),
        b'J' => Some((Key::KeyJ, true)),
        b'K' => Some((Key::KeyK, true)),
        b'L' => Some((Key::KeyL, true)),
        b'M' => Some((Key::KeyM, true)),
        b'N' => Some((Key::KeyN, true)),
        b'O' => Some((Key::KeyO, true)),
        b'P' => Some((Key::KeyP, true)),
        b'Q' => Some((Key::KeyQ, true)),
        b'R' => Some((Key::KeyR, true)),
        b'S' => Some((Key::KeyS, true)),
        b'T' => Some((Key::KeyT, true)),
        b'U' => Some((Key::KeyU, true)),
        b'V' => Some((Key::KeyV, true)),
        b'W' => Some((Key::KeyW, true)),
        b'X' => Some((Key::KeyX, true)),
        b'Y' => Some((Key::KeyY, true)),
        b'Z' => Some((Key::KeyZ, true)),

        b'0' => Some((Key::Num0, false)),
        b'1' => Some((Key::Num0, false)),
        b'2' => Some((Key::Num2, false)),
        b'3' => Some((Key::Num3, false)),
        b'4' => Some((Key::Num4, false)),
        b'5' => Some((Key::Num5, false)),
        b'6' => Some((Key::Num6, false)),
        b'7' => Some((Key::Num7, false)),
        b'8' => Some((Key::Num8, false)),
        b'9' => Some((Key::Num9, false)),

        b'!' => Some((Key::Num1, true)),
        b'@' => Some((Key::Num2, true)),
        b'#' => Some((Key::Num3, true)),
        b'$' => Some((Key::Num4, true)),
        b'%' => Some((Key::Num5, true)),
        b'^' => Some((Key::Num6, true)),
        b'&' => Some((Key::Num7, true)),
        b'*' => Some((Key::Num8, true)),
        b'(' => Some((Key::Num9, true)),
        b')' => Some((Key::Num0, true)),

        b'-' => Some((Key::Minus, false)),
        b'_' => Some((Key::Minus, true)),
        b'=' => Some((Key::Equal, false)),
        b'+' => Some((Key::Equal, true)),

        b'[' => Some((Key::LeftBracket, false)),
        b'{' => Some((Key::LeftBracket, true)),
        b']' => Some((Key::RightBracket, false)),
        b'}' => Some((Key::RightBracket, true)),

        b'\\' => Some((Key::BackSlash, false)),
        b'|' => Some((Key::BackSlash, true)),

        b';' => Some((Key::SemiColon, false)),
        b':' => Some((Key::SemiColon, true)),

        b'\'' => Some((Key::Quote, false)),
        b'"' => Some((Key::Quote, true)),

        b'`' => Some((Key::BackQuote, false)),
        b'~' => Some((Key::BackQuote, true)),

        b',' => Some((Key::Comma, false)),
        b'<' => Some((Key::Comma, true)),
        b'.' => Some((Key::Dot, false)),
        b'>' => Some((Key::Dot, true)),
        b'/' => Some((Key::Slash, false)),
        b'?' => Some((Key::Slash, true)),

        b' ' => Some((Key::Space, false)),
        b'\n' => Some((Key::Return, false)),
        b'\r' => Some((Key::Return, false)),
        b'\t' => Some((Key::Tab, false)),
        b'\x1b' => Some((Key::Escape, false)),

        _ => None,
    }
}

fn paste(output: String) -> Result<()> {
    log::info!("User said: {output}");
    for c in output.bytes() {
        if let Some((key, shift)) = key_from_char(c) {
            if shift {
                simulate(&EventType::KeyPress(Key::ShiftLeft))?;
            }
            simulate(&EventType::KeyPress(key))?;
            simulate(&EventType::KeyRelease(key))?;
            if shift {
                simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
struct State {
    ctrl: bool,
    recording: bool,
}

async fn download_model() -> Result<PathBuf> {
    let api = ApiBuilder::from_env().build()?;
    let repo = api.model("ggerganov/whisper.cpp".to_string());
    let filename = repo.get("ggml-base.en.bin").await?;
    Ok(filename)
}
#[tokio::main]
async fn main() -> Result<()> {
    let model_path = download_model().await?;
    // Just warmup the lock
    simulate(&EventType::KeyPress(Key::ControlLeft))?;
    simulate(&EventType::KeyRelease(Key::ControlLeft))?;
    // Conditionally compile with jack if the feature is specified.
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
    // Manually check for flags. Can be passed through cargo with -- e.g.
    // cargo run --release --example beep --features jack -- --jack
    let host = if true {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    #[cfg(any(
        not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        )),
        not(feature = "jack")
    ))]
    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("Cannot find input device"))?;

    println!("Input device: {}", device.name()?);

    // let config = device.default_input_config()?;
    // let config = pportedStreamConfig {
    //     channels: 1,
    //     sample_format: SampleFormat::F32,
    //     sample_rate: 16000,
    //     buffer_size: BufferSize::Default,
    // };
    // Define the desired format
    let config = StreamConfig {
        channels: 1,                           // mono
        sample_rate: cpal::SampleRate(16_000), // 16kHz
        buffer_size: cpal::BufferSize::Default,
    };

    // Check for supported formats (optional, but good for fallback/debugging)
    let mut supported_configs = device.supported_input_configs()?;
    let has_desired_format = supported_configs.any(|f| {
        f.min_sample_rate().0 <= 16_000
            && f.max_sample_rate().0 >= 16_000
            && f.channels() == 1
            && f.sample_format() == SampleFormat::F32
    });

    if !has_desired_format {
        println!("Warning: Desired format not explicitly supported, stream may not work.");
    }
    println!("Default input config: {:?}", config);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 4 * 8,
        sample_format: hound::SampleFormat::Float,
    };
    // const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    let mut cache_dir: PathBuf = dirs::cache_dir().unwrap();
    cache_dir.push("whispering");
    std::fs::create_dir_all(&cache_dir)?;
    let mut path = cache_dir.clone();
    path.push("recorded.wav");
    let writer = hound::WavWriter::create(&path, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer2 = writer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };
    let stream = device.build_input_stream(
        &config.into(),
        move |data, _: &_| write_input_data::<f32, f32>(data, &writer2),
        err_fn,
        None,
    )?;

    // let stream = match config.sample_format() {
    //     cpal::SampleFormat::I8 => device.build_input_stream(
    //         &config.into(),
    //         move |data, _: &_| write_input_data::<i8, i8>(data, &writer2),
    //         err_fn,
    //         None,
    //     )?,
    //     cpal::SampleFormat::I16 => device.build_input_stream(
    //         &config.into(),
    //         move |data, _: &_| write_input_data::<i16, i16>(data, &writer2),
    //         err_fn,
    //         None,
    //     )?,
    //     cpal::SampleFormat::I32 => device.build_input_stream(
    //         &config.into(),
    //         move |data, _: &_| write_input_data::<i32, i32>(data, &writer2),
    //         err_fn,
    //         None,
    //     )?,
    //     cpal::SampleFormat::F32 => device.build_input_stream(
    //         &config.into(),
    //         move |data, _: &_| write_input_data::<f32, f32>(data, &writer2),
    //         err_fn,
    //         None,
    //     )?,
    //     sample_format => {
    //         return Err(anyhow::Error::msg(format!(
    //             "Unsupported sample format '{sample_format}'"
    //         )));
    //     }
    // };

    let (schan, mut rchan) = unbounded_channel();
    let _listener = tokio::task::spawn_blocking(move || {
        listen(move |event| {
            schan
                .send(event)
                .unwrap_or_else(|e| println!("Could not send event {:?}", e));
        })
        .expect("Could not listen");
    });

    let mut state = State {
        ctrl: false,
        recording: false,
    };

    while let Some(event) = rchan.recv().await {
        match event.event_type {
            EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                state.ctrl = true;
            }
            EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                state.ctrl = false;
            }
            EventType::KeyPress(Key::Space) => {
                if state.ctrl {
                    state.recording = true;
                    let new_writer = hound::WavWriter::create(&path, spec)?;
                    *writer.lock().unwrap() = Some(new_writer);
                    stream.play()?
                }
            }
            EventType::KeyRelease(Key::Space) => {
                if state.recording {
                    state.recording = false;
                    stream.pause()?;
                    writer.lock().unwrap().take().unwrap().finalize()?;
                    let output = run_whisper(&model_path, &path)?;
                    paste(output)?;
                    // Always end by pressing Return to submit
                    simulate(&EventType::KeyPress(Key::Return))?;
                    simulate(&EventType::KeyRelease(Key::Return))?;
                }
            }
            _ => (),
        }
    }
    Ok(())
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}

fn run_whisper(model_path: &PathBuf, wav_path: &PathBuf) -> Result<String> {
    install_logging_hooks();

    // load a context and model
    let ctx = WhisperContext::new_with_params(
        &model_path.display().to_string(),
        WhisperContextParameters::default(),
    )
    .expect("failed to load model");

    // create a params object
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // assume we have a buffer of audio data
    // here we'll make a fake one, floating point samples, 32 bit, 16KHz, mono
    let samples: Vec<f32> = hound::WavReader::open(wav_path)
        .unwrap()
        .into_samples::<f32>()
        .map(|x| x.unwrap())
        .collect();

    // now we can run the model
    let mut state = ctx.create_state().expect("failed to create state");
    state
        .full(params, &samples[..])
        .expect("failed to run model");

    // fetch the results
    let num_segments = state
        .full_n_segments()
        .expect("failed to get number of segments");
    let mut output = String::new();
    for i in 0..num_segments {
        let segment = state
            .full_get_segment_text(i)
            .expect("failed to get segment");
        output.push_str(&segment);
    }
    Ok(output)
}
