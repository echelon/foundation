/**
 * Per-model request snippets for the model page's API section — cURL, JS fetch,
 * and Python requests. Mirrors the real endpoint shapes (`api_*` design docs):
 * chat models hit /v1/chat/completions, everything else the model-specific
 * /v1/models/{slug}/generate.
 */
import { type Model } from "./catalog";
import { type CodeLang } from "@/components/CodeBlock";

const BASE = "https://api.foundation.ai";

/** Example body params per modality, kept short for readability. */
function bodyParams(model: Model): Record<string, unknown> {
  switch (model.modality) {
    case "video":
      return { prompt: "a serene mountain lake at dawn, slow drone push-in", duration: 5, resolution: "1080p" };
    case "3d":
      return { prompt: "a low-poly treasure chest, game asset" };
    case "world":
      return { prompt: "an explorable misty forest clearing" };
    case "image":
    default:
      return { prompt: "a serene mountain lake at dawn, cinematic", aspect_ratio: "16:9" };
  }
}

function isChat(model: Model) {
  return model.modality === "chat";
}

function endpoint(model: Model): string {
  return isChat(model) ? `${BASE}/v1/chat/completions` : `${BASE}/v1/models/${model.slug}/generate`;
}

function jsonBody(model: Model): Record<string, unknown> {
  if (isChat(model)) {
    return {
      model: model.slug,
      messages: [{ role: "user", content: "Explain quantum entanglement in one sentence." }],
      max_tokens: 512,
    };
  }
  return bodyParams(model);
}

/** result-access expression per language, tailored to the response shape. */
function resultJs(model: Model): string {
  return isChat(model) ? "data.choices[0].message.content" : "data.output[0].url";
}
function resultPy(model: Model): string {
  return isChat(model) ? 'data["choices"][0]["message"]["content"]' : 'data["output"][0]["url"]';
}

/** Pretty JSON body, embedded verbatim by the raw-string languages. */
function bodyJson(model: Model): string {
  return JSON.stringify(jsonBody(model), null, 2);
}

export function curlSnippet(model: Model): string {
  const body = JSON.stringify(jsonBody(model), null, 2)
    .split("\n")
    .map((l, i) => (i === 0 ? l : "  " + l))
    .join("\n");
  return `curl ${endpoint(model)} \\
  -H "Authorization: Bearer $FOUNDATION_KEY" \\
  -H "Content-Type: application/json" \\
  -d '${body}'`;
}

export function jsSnippet(model: Model): string {
  const body = JSON.stringify(jsonBody(model), null, 2)
    .split("\n")
    .map((l, i) => (i === 0 ? l : "    " + l))
    .join("\n");
  return `const res = await fetch("${endpoint(model)}", {
  method: "POST",
  headers: {
    Authorization: \`Bearer \${process.env.FOUNDATION_KEY}\`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify(${body}),
});

const data = await res.json();
console.log(${resultJs(model)});`;
}

export function pySnippet(model: Model): string {
  const body = JSON.stringify(jsonBody(model), null, 4)
    .split("\n")
    .map((l, i) => (i === 0 ? l : "    " + l))
    .join("\n")
    .replace(/: true/g, ": True")
    .replace(/: false/g, ": False");
  return `import os
import requests

res = requests.post(
    "${endpoint(model)}",
    headers={"Authorization": f"Bearer {os.environ['FOUNDATION_KEY']}"},
    json=${body},
)
data = res.json()
print(${resultPy(model)})`;
}

export function phpSnippet(model: Model): string {
  return `<?php

$body = <<<JSON
${bodyJson(model)}
JSON;

$ch = curl_init("${endpoint(model)}");
curl_setopt_array($ch, [
    CURLOPT_RETURNTRANSFER => true,
    CURLOPT_POST => true,
    CURLOPT_HTTPHEADER => [
        "Authorization: Bearer " . getenv("FOUNDATION_KEY"),
        "Content-Type: application/json",
    ],
    CURLOPT_POSTFIELDS => $body,
]);

echo curl_exec($ch);`;
}

export function csharpSnippet(model: Model): string {
  return `using System;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;

class Program
{
    static async Task Main()
    {
        var body = """
${bodyJson(model)}
""";

        using var client = new HttpClient();
        var request = new HttpRequestMessage(HttpMethod.Post, "${endpoint(model)}");
        request.Headers.Add("Authorization", $"Bearer {Environment.GetEnvironmentVariable("FOUNDATION_KEY")}");
        request.Content = new StringContent(body, Encoding.UTF8, "application/json");

        var response = await client.SendAsync(request);
        Console.WriteLine(await response.Content.ReadAsStringAsync());
    }
}`;
}

export function javaSnippet(model: Model): string {
  return `import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;

public class Example {
    public static void main(String[] args) throws Exception {
        String body = """
${bodyJson(model)}
""";

        HttpClient client = HttpClient.newHttpClient();
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create("${endpoint(model)}"))
                .header("Authorization", "Bearer " + System.getenv("FOUNDATION_KEY"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(body))
                .build();

        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
        System.out.println(response.body());
    }
}`;
}

export function goSnippet(model: Model): string {
  return `package main

import (
    "bytes"
    "fmt"
    "io"
    "net/http"
    "os"
)

func main() {
    body := []byte(\`${bodyJson(model)}\`)

    req, _ := http.NewRequest("POST", "${endpoint(model)}", bytes.NewBuffer(body))
    req.Header.Set("Authorization", "Bearer "+os.Getenv("FOUNDATION_KEY"))
    req.Header.Set("Content-Type", "application/json")

    res, err := http.DefaultClient.Do(req)
    if err != nil {
        panic(err)
    }
    defer res.Body.Close()

    out, _ := io.ReadAll(res.Body)
    fmt.Println(string(out))
}`;
}

export function rustSnippet(model: Model): string {
  return `use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body = r#"${bodyJson(model)}"#;

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("${endpoint(model)}")
        .bearer_auth(env::var("FOUNDATION_KEY")?)
        .header("Content-Type", "application/json")
        .body(body)
        .send()?;

    println!("{}", res.text()?);
    Ok(())
}`;
}

export function kotlinSnippet(model: Model): string {
  return `import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

fun main() {
    val body = """
${bodyJson(model)}
""".trimIndent().toRequestBody("application/json".toMediaType())

    val request = Request.Builder()
        .url("${endpoint(model)}")
        .header("Authorization", "Bearer \${System.getenv("FOUNDATION_KEY")}")
        .post(body)
        .build()

    OkHttpClient().newCall(request).execute().use { response ->
        println(response.body?.string())
    }
}`;
}

export function swiftSnippet(model: Model): string {
  return `import Foundation

let body = """
${bodyJson(model)}
"""

let key = ProcessInfo.processInfo.environment["FOUNDATION_KEY"] ?? ""
var request = URLRequest(url: URL(string: "${endpoint(model)}")!)
request.httpMethod = "POST"
request.setValue("Bearer \\(key)", forHTTPHeaderField: "Authorization")
request.setValue("application/json", forHTTPHeaderField: "Content-Type")
request.httpBody = body.data(using: .utf8)

let task = URLSession.shared.dataTask(with: request) { data, _, _ in
    if let data = data {
        print(String(data: data, encoding: .utf8)!)
    }
}
task.resume()`;
}

/** All request snippets for a model, in display order. */
export function snippets(model: Model): { label: string; lang: CodeLang; code: string }[] {
  return [
    { label: "cURL", lang: "bash", code: curlSnippet(model) },
    { label: "JavaScript", lang: "javascript", code: jsSnippet(model) },
    { label: "Python", lang: "python", code: pySnippet(model) },
    { label: "PHP", lang: "php", code: phpSnippet(model) },
    { label: "C#", lang: "csharp", code: csharpSnippet(model) },
    { label: "Java", lang: "java", code: javaSnippet(model) },
    { label: "Go", lang: "go", code: goSnippet(model) },
    { label: "Rust", lang: "rust", code: rustSnippet(model) },
    { label: "Kotlin", lang: "kotlin", code: kotlinSnippet(model) },
    { label: "Swift", lang: "swift", code: swiftSnippet(model) },
  ];
}
