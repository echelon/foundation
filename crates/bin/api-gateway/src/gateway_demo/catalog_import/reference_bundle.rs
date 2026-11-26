mod stage;

use std::{
    ffi::OsString,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{Certificate, Client, Url};
use serde::{Deserialize, Serialize};

use super::{digest, read_file, read_limited, reference_signature, Reference, MAX_CONFIG_BYTES};

const MAX_BUNDLE_BYTES: usize = 96 * 1024;
const HELP: &str = "Usage:
  api-gateway catalog pack-reference --reference FILE --signature FILE --public-key FILE
  api-gateway catalog stage-reference --reference FILE --signature FILE --public-key FILE --directory DIR
  api-gateway catalog fetch-reference --registry HTTPS_URL (--sha256 HEX | --current) --public-key FILE [--ca FILE]

Pack a checked signature and its snapshot into one file.
Stage the immutable and current copies in a local directory.
Fetch a bundle from an HTTPS registry and verify it before writing to stdout.
";

pub(super) enum Action {
    Pack,
    Stage,
    Fetch,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    version: u32,
    reference_base64: String,
    signature: reference_signature::SignatureFile,
}

struct FetchOptions<'a> {
    registry: &'a str,
    checksum: Option<&'a str>,
    public_key: &'a [u8],
    ca: Option<&'a Path>,
}

pub(super) async fn run(action: Action, mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut reference, mut signature, mut key, mut registry, mut checksum, mut ca, mut directory) =
        (None, None, None, None, None, None, None);
    let mut current = false;
    while let Some(option) = args.next() {
        let slot = match (&action, option.to_str()) {
            (_, Some("--public-key")) => &mut key,
            (Action::Pack | Action::Stage, Some("--reference")) => &mut reference,
            (Action::Pack | Action::Stage, Some("--signature")) => &mut signature,
            (Action::Stage, Some("--directory")) => &mut directory,
            (Action::Fetch, Some("--registry")) => &mut registry,
            (Action::Fetch, Some("--sha256")) => &mut checksum,
            (Action::Fetch, Some("--ca")) => &mut ca,
            (Action::Fetch, Some("--current")) => {
                ensure!(!current, "duplicate reference bundle option");
                current = true;
                continue;
            }
            (_, Some("--help" | "-h")) => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown reference bundle option. Use --help"),
        };
        let value = args
            .next()
            .context("reference bundle option needs a value")?;
        ensure!(
            slot.replace(value).is_none(),
            "duplicate reference bundle option"
        );
    }
    let key = PathBuf::from(key.context("set --public-key FILE")?);
    let public_key = read_file(&key)?;
    reference_signature::public_key_bytes(&public_key)?;
    let bytes = match action {
        Action::Pack | Action::Stage => {
            let source = Reference::parse(read_file(Path::new(
                &reference.context("set --reference FILE")?,
            ))?)?;
            let signature = reference_signature::read_signature(Path::new(
                &signature.context("set --signature FILE")?,
            ))?;
            let verified = reference_signature::verify(&source.bytes, &source.catalog, &signature, &public_key)?;
            let bundle = Bundle {
                version: 1,
                reference_base64: STANDARD.encode(&source.bytes),
                signature,
            };
            let mut bytes = serde_json::to_vec_pretty(&bundle)?;
            bytes.push(b'\n');
            ensure!(
                bytes.len() <= MAX_BUNDLE_BYTES,
                "reference bundle exceeds 96 KiB"
            );
            if matches!(action, Action::Stage) {
                let directory = directory.context("set --directory DIR")?;
                let placed = stage::write(Path::new(&directory), &verified, &public_key, &bytes)?;
                let mut output = io::stdout().lock();
                serde_json::to_writer_pretty(&mut output, &placed)?;
                writeln!(output)?;
                return Ok(());
            }
            bytes
        }
        Action::Fetch => {
            let registry = registry
                .as_deref()
                .and_then(|value| value.to_str())
                .context("set --registry to an HTTPS directory URL")?;
            ensure!(current != checksum.is_some(), "choose exactly one of --sha256 or --current");
            let checksum = checksum.as_deref().map(|value| {
                value.to_str().filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
                    .map(str::to_ascii_lowercase)
                    .context("set --sha256 to the 64-digit reference SHA-256")
            }).transpose()?;
            fetch(FetchOptions {
                registry,
                checksum: checksum.as_deref(),
                public_key: &public_key,
                ca: ca.as_deref().map(Path::new),
            })
            .await?
        }
    };
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}

pub(super) fn load(path: &Path, public_key: &Path) -> Result<Reference> {
    let bytes = read_limited(path, MAX_BUNDLE_BYTES as u64)?;
    verify_bundle(&bytes, &read_file(public_key)?)
}

fn verify_bundle(bytes: &[u8], public_key: &[u8]) -> Result<Reference> {
    let bundle: Bundle =
        serde_json::from_slice(bytes).context("decode ArtCraft reference bundle")?;
    ensure!(bundle.version == 1, "unsupported reference bundle version");
    let raw = STANDARD
        .decode(bundle.reference_base64)
        .context("decode bundled reference bytes")?;
    ensure!(
        raw.len() as u64 <= MAX_CONFIG_BYTES,
        "bundled catalog exceeds 64 KiB"
    );
    let mut reference = Reference::parse(raw)?;
    reference.verified = Some(reference_signature::verify(
        &reference.bytes,
        &reference.catalog,
        &bundle.signature,
        public_key,
    )?);
    Ok(reference)
}

async fn fetch(options: FetchOptions<'_>) -> Result<Vec<u8>> {
    let FetchOptions {
        registry,
        checksum,
        public_key,
        ca,
    } = options;
    let registry = Url::parse(registry).map_err(|_| anyhow::anyhow!("invalid registry URL"))?;
    ensure!(
        registry.scheme() == "https"
            && registry.host_str().is_some()
            && registry.path().ends_with('/')
            && registry.username().is_empty()
            && registry.password().is_none()
            && registry.query().is_none()
            && registry.fragment().is_none(),
        "registry URL must be an HTTPS directory without credentials, a query, or a fragment"
    );
    let release = checksum.unwrap_or("current");
    let url = registry.join(&format!("{release}/{}.json", digest(public_key)))?;
    let mut client = Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(12));
    if let Some(ca) = ca {
        let root =
            Certificate::from_pem(&read_file(ca)?).context("invalid registry CA certificate")?;
        client = client
            .tls_built_in_root_certs(false)
            .add_root_certificate(root);
    }
    let mut request = client
        .build()?
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json");
    if checksum.is_none() {
        request = request.header(reqwest::header::CACHE_CONTROL, "no-cache");
    }
    let mut response = request.send()
        .await
        .context("fetch ArtCraft reference bundle")?;
    ensure!(
        response.status().is_success(),
        "release registry returned {}",
        response.status()
    );
    ensure!(
        response
            .content_length()
            .is_none_or(|size| size <= MAX_BUNDLE_BYTES as u64),
        "reference bundle exceeds 96 KiB"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            chunk.len() <= MAX_BUNDLE_BYTES - bytes.len(),
            "reference bundle exceeds 96 KiB"
        );
        bytes.extend_from_slice(&chunk);
    }
    let reference = verify_bundle(&bytes, public_key)?;
    let verified = reference
        .verified
        .context("reference bundle was not verified")?;
    if let Some(checksum) = checksum {
        ensure!(
            verified.reference_sha256 == checksum,
            "bundle does not match the requested reference SHA-256"
        );
    }
    Ok(bytes)
}
