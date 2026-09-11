use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin";
const MODEL_SHA256: &str = "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7";

/// Downloads the multilingual `tiny` Whisper model into a cache shared
/// across builds, so it survives `cargo clean` and isn't re-fetched on
/// every compile. `main.rs` embeds it via `include_bytes!` at this path.
fn main() {
    let cache_dir = model_cache_dir();
    fs::create_dir_all(&cache_dir).expect("failed to create model cache dir");
    let model_path = cache_dir.join("ggml-tiny-q5_1.bin");

    if !model_is_valid(&model_path) {
        download_model(&model_path);
        assert!(
            model_is_valid(&model_path),
            "downloaded model failed checksum verification"
        );
    }

    println!(
        "cargo:rustc-env=CAPCAP_TINY_MODEL_PATH={}",
        model_path.display()
    );
    println!("cargo:rerun-if-changed={}", model_path.display());
}

fn model_cache_dir() -> PathBuf {
    // Shared across the whole workspace, not OUT_DIR, so a fresh
    // OUT_DIR per build doesn't force a 30MB re-download every time.
    // Absolute, since `include_bytes!` resolves relative paths against
    // the source file that calls it, not the build's working directory.
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    manifest_dir.join(target_dir).join("model-cache")
}

fn model_is_valid(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return false;
    }
    sha256_hex(&bytes) == MODEL_SHA256
}

fn download_model(dest: &Path) {
    eprintln!("capcap: downloading bundled tiny Whisper model ({MODEL_URL})");
    let response = ureq::get(MODEL_URL)
        .call()
        .expect("failed to download bundled Whisper model");
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .read_to_end(&mut bytes)
        .expect("failed to read downloaded model body");
    fs::write(dest, &bytes).expect("failed to write downloaded model to cache");
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
