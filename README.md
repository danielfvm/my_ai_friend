# My AI Friend
Ever wanted a friend that is always here to listen to you No? Well then maybe it is just me. Anyways this was just a quick experiment where I combined
piper, ollama and whisper to make an interactive chatbot that I can talk to.

## How it works
It uses whisper for speech to text and then uses the text as a prompt for a LLM model running in ollama. The output is then again converted to speech using piper.
That's it.

## Tools
The LLM Model can use tools to solve tasks, or simply communicate to the outside world. Currently I placed all the tools [here](/src/tools). If you write your own
tools, make sure to add them to the list in the `main.rs` file. Keep in mind not all AI models support tools!!! One that does support tools is `llama3.1:8b`.


## Setup
You will need to download the models for whisper, ollama and piper separately.

### Whisper
To get whisper working you will need to download the correct AI model for the language you want to use. You can automatically download it by running the following command:
```
./download-ggml-model.sh
```

For reference, I am using the following model: `ggml-base.en.bin`

### Piper
For piper you will need to download the voice model you want to use (Again language dependent). You can download pre trained models [here](https://huggingface.co/rhasspy/piper-voices/tree/main).
You will need the `.onnx` and `.onnx.json` file. If you want to tweak the voice slightly, edit the `.onnx.json` file. I for instance tweaked `sample_rate` and `length_scale` to change the pitch a little.
Also you can train your [own voices](https://github.com/rhasspy/piper/blob/master/TRAINING.md).

### Ollama
Finally the brain of the bot, ollama. If you haven't yet, you can install ollama from the [offical website](https://ollama.com/) and then download a model that you want to try (and that your computer can handle).
Keep in mind that not all models support using `tools`. 

Here a list of models that I was playing around with:

* llama3.1:8b
* gemma3:12b
* qwen3:8b
* mistral:7b

### Configuration
After you have downloaded all the models, you will need to edit the `config.json` to include the path to the models you have downloaded.
You might also need to change the `silence_threshold` to fit with your microphone. There is no built-in way to see that value right now though.

### Run
Finally run the program with
```
cargo run
```
and start speaking.

## Disclaimer / ToDo
Work in progress. Also I'm bad at Rust so the code is rly ugly.

## License
MIT
