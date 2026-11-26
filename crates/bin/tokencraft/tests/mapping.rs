use std::path::{Path, PathBuf};

use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use tokio::{net::TcpListener, process::Command};

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../examples/mapping")
        .join(name)
}

#[tokio::test]
async fn ai_draft_is_a_validatable_file_and_approval_replays_both_samples() {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../examples/mapping/example-image.map.json"
    ))
    .unwrap();
    let rules = file["rules"].clone();
    let server = Router::new().route(
        "/v1/chat/completions",
        post(move |Json(request): Json<Value>| {
            let rules = rules.clone();
            async move {
                assert_eq!(request["model"], "fake-mapper");
                let prompt = request["messages"][1]["content"].as_str().unwrap();
                let context: Value = serde_json::from_str(prompt).unwrap();
                assert_eq!(context["schema"]["primitive"], "image");
                assert!(context["documentation"]
                    .as_str()
                    .unwrap()
                    .contains("layout"));
                Json(json!({"choices":[{"message":{"content":rules.to_string()}}]}))
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async { axum::serve(listener, server).await.unwrap() });

    let output = Command::new(env!("CARGO_BIN_EXE_tokencraft"))
        .args([
            "mapping",
            "draft",
            "--provider",
            "self_hosted",
            "--model",
            "example-image",
            "--primitive",
            "image",
            "--context",
        ])
        .arg(example("provider-context.md"))
        .env("TOKENCRAFT_MAPPER_ENDPOINT", endpoint)
        .env("TOKENCRAFT_MAPPER_MODEL", "fake-mapper")
        .env_remove("TOKENCRAFT_MAPPER_API_KEY")
        .output()
        .await
        .unwrap();
    server.abort();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let draft: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(draft["status"], "draft");
    assert_eq!(draft["source"]["sha256"].as_str().unwrap().len(), 64);

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let scratch = Scratch(
        std::env::temp_dir().join(format!("tokencraft-mapping-{}.json", rand::random::<u64>())),
    );
    std::fs::write(&scratch.0, &output.stdout).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_tokencraft"))
        .args(["mapping", "approve", "--file"])
        .arg(&scratch.0)
        .arg("--request")
        .arg(example("request.json"))
        .arg("--response")
        .arg(example("response.json"))
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["status"],
        "approved"
    );
}
