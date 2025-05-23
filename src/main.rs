extern crate chrono;
mod tools;

use ollama_rs::Ollama;
use ollama_rs::coordinator::Coordinator;
use ollama_rs::generation::chat::ChatMessage;
use piper_rs::synth::PiperSpeechSynthesizer;
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use regex::Regex;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::tools::time::TimeTool;
use crate::tools::timeout::TimeoutTool;

/**
 * Removes the <think></think> tags used in AI models like Gwen and DeepSeek
 **/
fn remove_think_tags(input: &str) -> String {
    let i = input
        .find("</think>")
        .and_then(|i| Some(i + 8))
        .unwrap_or(0);
    String::from(&input[i..])
}

/**
 * Remove all emoji from a string
 * This is needed because I don't want emojis to be read by Whisper
 **/
fn remove_emoji(string: String) -> String {
    let regex = Regex::new(concat!(
        "[",
        "\u{01F600}-\u{01F64F}", // emoticons
        "\u{01F300}-\u{01F5FF}", // symbols & pictographs
        "\u{01F680}-\u{01F6FF}", // transport & map symbols
        "\u{01F1E0}-\u{01F1FF}", // flags (iOS)
        "\u{002702}-\u{0027B0}",
        "\u{0024C2}-\u{01F251}",
        "]+",
    ))
    .unwrap();

    regex.replace_all(&string, "").to_string()
}

/**
 * Downsample the audio to 16kHz
 * This is needed because the Whisper model expects 16kHz audio
 **/
fn downsample_to_16k(input: &[f32], input_rate: usize) -> Vec<f32> {
    let ratio = input_rate as f32 / 16_000.0;
    input
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as f32 % ratio).round() == 0.0)
        .map(|(_, &v)| v)
        .collect()
}

#[derive(Serialize, Deserialize)]
struct Config {
    system: String, // System prompt used for the AI
    ollama: String, // Model used for the Ollama AI
    whisper: String,
    piper: String,

    silence_threshold: f32, // Volume level to be considered silence
    silence_duration: u64,  // The duration of silence necessary to trigger the AI in milliseconds

    use_tools: bool, // Some LLMs dont support tools, set to false if you still want to use them
}

#[tokio::main]
async fn main() {
    let cfg: Config = serde_json::from_str(include_str!("../config.json")).unwrap();

    // load a context and model
    let ctx = WhisperContext::new_with_params(&cfg.whisper, WhisperContextParameters::default())
        .expect("failed to load model");

    let model = piper_rs::from_config_path(Path::new(&cfg.piper))
        .expect("Failed to load config file for Piper model");

    let synth = PiperSpeechSynthesizer::new(model).expect("Failed to load Piper model");

    // Shared state
    let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let has_talked = Arc::new(Mutex::new(false));
    let last_voice_time = Arc::new(Mutex::new(Instant::now()));

    // Setup CPAL
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config().unwrap();
    let sample_rate = config.sample_rate().0 as usize;
    println!("Sample rate: {}", sample_rate);
    println!("Input device: {:?}", device.name());

    // Currently only f32 is supported, should be easy to add support for other formats though
    assert_eq!(
        config.sample_format(),
        cpal::SampleFormat::F32,
        "Only f32 is supported right now."
    );

    let speech_buffer_clone = Arc::clone(&speech_buffer);
    let has_talked_clone = Arc::clone(&has_talked);
    let last_voice_time_clone = Arc::clone(&last_voice_time);

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                let mut buffer = speech_buffer_clone.lock().unwrap();
                let mut has_talked = has_talked_clone.lock().unwrap();
                let mut last_time = last_voice_time_clone.lock().unwrap();

                buffer.extend_from_slice(data);

                // Append samples and update last_audio_time if not silent
                let rms = data.iter().map(|s| s * s).sum::<f32>() / data.len() as f32;
                if rms > cfg.silence_threshold {
                    *last_time = Instant::now();
                    *has_talked = true;
                }

                println!("{}", rms);
            },
            |err| eprintln!("Stream error: {:?}", err),
            Some(Duration::from_millis(cfg.silence_duration)),
        )
        .unwrap();

    stream.play().unwrap();
    println!("Listening with VAD...");

    // Open the default audio output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    // Create a sink (something that plays audio)
    let sink = Sink::try_new(&stream_handle).unwrap();

    // By default, it will connect to localhost:11434
    let ollama = Ollama::default();
    let history = vec![ChatMessage {
        role: ollama_rs::generation::chat::MessageRole::System,
        content: cfg.system,
        tool_calls: vec![],
        images: None,
    }];

    let timeout = Arc::new(Mutex::new(Instant::now()));

    let mut coordinator = Coordinator::new(ollama, cfg.ollama, history);

    // TODO: Add other tools that the AI should use here:
    if cfg.use_tools {
        coordinator = coordinator
            .add_tool(TimeoutTool {
                timeout: timeout.clone(),
            })
            .add_tool(TimeTool {});
    }

    loop {
        std::thread::sleep(Duration::from_millis(100));

        let elapsed = last_voice_time.lock().unwrap().elapsed();

        if elapsed > Duration::from_millis(cfg.silence_duration) {
            let mut buffer = speech_buffer.lock().unwrap();
            let mut has_talked = has_talked.lock().unwrap();

            if *has_talked {
                println!("Silence detected — transcribing...");

                // Transcribe with Whisper
                let mut state = ctx.create_state().unwrap();
                let params = FullParams::new(SamplingStrategy::Greedy { best_of: 3 });

                let k16 = downsample_to_16k(&buffer, sample_rate);
                state.full(params, &k16).expect("Whisper failed");

                let prompt = state.full_get_segment_text(0).unwrap_or_default();
                println!("Transcription: {}", prompt);

                buffer.clear();
                *has_talked = false;

                if (*timeout.lock().unwrap())
                    .duration_since(Instant::now())
                    .as_secs()
                    > 0
                    && !prompt.to_lowercase().contains(TimeoutTool::MAGIC_WORD)
                {
                    println!("Timeout");
                } else {
                    *timeout.lock().unwrap() = Instant::now();

                    // Ask ollama to generate a response, it might use a tool here
                    let Ok(res) = coordinator.chat(vec![ChatMessage::user(prompt)]).await else {
                        println!("Error failed to get response from AI");
                        continue;
                    };

                    let result = res.message.content;
                    println!("Response: {}", result);

                    let output_path = Path::new("output.wav");

                    synth
                        .synthesize_to_file(
                            output_path,
                            remove_emoji(remove_think_tags(&result)),
                            None,
                        )
                        .expect("Failed to synthesize speech");

                    // Play the generated oudio file
                    let file = File::open(output_path).expect("Failed to open file");
                    let source =
                        Decoder::new(BufReader::new(file)).expect("Failed to decode audio file");

                    sink.append(source);
                    sink.sleep_until_end();
                }
            }
        }
    }
}
