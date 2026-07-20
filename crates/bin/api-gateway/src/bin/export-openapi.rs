//! Print the gateway's OpenAPI spec as pretty JSON on stdout (drives `just api-docs`).

use utoipa::OpenApi;

fn main() {
  println!("{}", api_gateway::openapi::ApiDoc::openapi().to_pretty_json().expect("openapi document serializes"));
}
