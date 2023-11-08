use anyhow::Error as E;
use candle_core::{DType, Device, Result, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_nn::VarBuilder;
use candle_transformers::{
    generation::LogitsProcessor,
    models::{blip, quantized_blip},
};
use serde_json::json;
use tokenizers::Tokenizer;

#[derive(Debug, Clone)]
pub enum Model {
    M(blip::BlipForConditionalGeneration),
    Q(quantized_blip::BlipForConditionalGeneration),
}

impl Model {
    fn text_decoder_forward(&mut self, xs: &Tensor, img_xs: &Tensor) -> Result<Tensor> {
        match self {
            Self::M(m) => m.text_decoder().forward(xs, img_xs),
            Self::Q(m) => m.text_decoder().forward(xs, img_xs),
        }
    }

    fn embed_image(&mut self, image: &Tensor) -> Result<Tensor> {
        match self {
            Self::M(m) => image.unsqueeze(0)?.apply(m.vision_model()),
            Self::Q(m) => image.unsqueeze(0)?.apply(m.vision_model()),
        }
    }

    fn reset_kv_cache(&mut self) {
        match self {
            Self::M(m) => m.reset_kv_cache(),
            Self::Q(m) => m.reset_kv_cache(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelArgs {
    pub model: Option<String>,
    pub tokenizer: Option<String>,
    pub cpu: bool,
    pub quantized: bool,
}

pub struct ModelResources {
    pub model: Model,
    pub tokenizer: TokenOutputStream,
    pub logit_processor: LogitsProcessor,
}

impl ModelResources {
    pub fn new(args: ModelArgs) -> anyhow::Result<Self> {
        let (model, tokenizer, logit_processor) = model(args)?;
        Ok(Self {
            model,
            tokenizer,
            logit_processor,
        })
    }
}

const SEP_TOKEN_ID: u32 = 102;

// Read image from bytes and resize it to 384x384 and return a tensor with shape (3, 384, 384).
pub fn load_image_from_bytes(bytes: &[u8]) -> Result<Tensor> {
    let img = image::load_from_memory(bytes).map_err(candle_core::Error::wrap)?;
    let img = img.resize_to_fill(384, 384, image::imageops::FilterType::Triangle);
    let img = img.to_rgb8();
    let data = img.into_raw();
    let data = Tensor::from_vec(data, (384, 384, 3), &Device::Cpu)?.permute((2, 0, 1))?;
    let mean =
        Tensor::new(&[0.48145466f32, 0.4578275, 0.40821073], &Device::Cpu)?.reshape((3, 1, 1))?;
    let std = Tensor::new(&[0.26862954f32, 0.261_302_6, 0.275_777_1], &Device::Cpu)?
        .reshape((3, 1, 1))?;
    (data.to_dtype(candle_core::DType::F32)? / 255.)?
        .broadcast_sub(&mean)?
        .broadcast_div(&std)
}

pub fn model(args: ModelArgs) -> anyhow::Result<(Model, TokenOutputStream, LogitsProcessor)> {
    println!("Downloading model...");
    let model_file = match args.model {
        None => {
            let api = hf_hub::api::sync::Api::new()?;
            if args.quantized {
                let api = api.model("lmz/candle-blip".to_string());
                api.get("blip-image-captioning-large-q4k.gguf")?
            } else {
                let api = api.repo(hf_hub::Repo::with_revision(
                    "Salesforce/blip-image-captioning-large".to_string(),
                    hf_hub::RepoType::Model,
                    "refs/pr/18".to_string(),
                ));
                api.get("model.safetensors")?
            }
        }
        Some(model) => model.into(),
    };
    println!("Loading tokenizer...");
    let tokenizer = match args.tokenizer {
        None => {
            let api = hf_hub::api::sync::Api::new()?;
            let api = api.model("Salesforce/blip-image-captioning-large".to_string());
            api.get("tokenizer.json")?
        }
        Some(file) => file.into(),
    };
    let tokenizer = Tokenizer::from_file(tokenizer).map_err(E::msg)?;
    let tokenizer = TokenOutputStream::new(tokenizer);
    let logits_processor = candle_transformers::generation::LogitsProcessor::new(1337, None, None);
    let config = blip::Config::image_captioning_large();
    let device = Device::Cpu;
    println!("Loading model...");
    let model = if args.quantized {
        let vb = quantized_blip::VarBuilder::from_gguf(model_file)?;
        let model = quantized_blip::BlipForConditionalGeneration::new(&config, vb)?;
        Model::Q(model)
    } else {
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[model_file], DType::F32, &device)? };
        let model = blip::BlipForConditionalGeneration::new(&config, vb)?;
        Model::M(model)
    };
    println!("Model and tokenizer loaded.");
    Ok((model, tokenizer, logits_processor))
}

pub async fn detect(
    data: &mut ModelResources,
    image: &[u8],
    sender: tokio::sync::mpsc::Sender<std::string::String>,
) -> anyhow::Result<String> {
    let device = Device::Cpu;

    println!("Loading image...");
    crate::send_and_wait(
        sender.clone(),
        json!({"status": "Loading model..."}).to_string(),
    ).await;
    let image = load_image_from_bytes(image)?.to_device(&device)?;

    println!("Generating caption...");
    crate::send_and_wait(
        sender.clone(),
        json!({"status": "Loading image..."}).to_string(),
    ).await;

    let model = &mut data.model;
    let tokenizer = &mut data.tokenizer;
    let logits_processor = &mut data.logit_processor;

    println!("Embedding image...");
    crate::send_and_wait(
        sender.clone(),
        json!({"status": "Generating caption..."}).to_string(),
    ).await;

    let image_embeds = model.embed_image(&image)?;
    let mut token_ids = vec![30522u32];

    println!("Generating tokens...");
    crate::send_and_wait(
        sender.clone(),
        json!({"status": "Generating tokens..."}).to_string(),
    ).await;

    for index in 0..1000 {
        let context_size = if index > 0 { 1 } else { token_ids.len() };
        let start_pos = token_ids.len().saturating_sub(context_size);
        let input_ids = Tensor::new(&token_ids[start_pos..], &device)?.unsqueeze(0)?;
        let logits = model.text_decoder_forward(&input_ids, &image_embeds)?;
        let logits = logits.squeeze(0)?;
        let logits = logits.get(logits.dim(0)? - 1)?;
        let token = logits_processor.sample(&logits)?;
        if token == SEP_TOKEN_ID {
            break;
        }
        token_ids.push(token);
        // decode the token
        if let Some(token) = tokenizer.next_token(token)? {
            println!(
                "Sending token: {} at time {}",
                token,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            );
            sender
                .send(
                    json!({
                        "status": "token",
                        "token": token,
                    })
                    .to_string(),
                )
                .await
                .unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    println!("Resetting cache...");
    model.reset_kv_cache();
    let final_output = token_ids
        .iter()
        .filter_map(|&x| tokenizer.next_token(x).ok().unwrap())
        .collect::<Vec<_>>()
        .join("");
    Ok(final_output)
}
