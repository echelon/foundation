mod draft;

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure, Context, Result};
use provider_mapping::{Direction, Mapping, Registry, Status};
use serde::Serialize;
use serde_json::Value;

const HELP: &str = "Usage: tokencraft mapping COMMAND [OPTIONS]

  schema   [--primitive ID]                          Print the standard or one primitive
  list     --dir DIR                                 List mapping files and revisions
  validate --file MAPPING                            Check rule structure and schema paths
  preview  --file MAPPING --direction request|response --input SAMPLE
  apply    --file MAPPING --provider ID --model ID --direction request|response --input SAMPLE
  approve  --file MAPPING --request SAMPLE --response SAMPLE
  draft    --primitive ID --provider ID --model ID [--revision ID]
           (--context FILE | --url HTTPS_URL)        Propose a draft via a configured model

Every command accepts --schema FILE for a local registry override.
Set TOKENCRAFT_MAPPER_ENDPOINT and TOKENCRAFT_MAPPER_MODEL to use draft.
Set TOKENCRAFT_MAPPER_API_KEY when the endpoint requires a bearer token.
JSON results are written to stdout.
";

const MAX_FILE: u64 = 256 * 1024;

pub(crate) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let command = args.next().unwrap_or_else(|| "--help".into());
    if matches!(command.to_str(), Some("help" | "--help" | "-h")) {
        print!("{HELP}");
        return Ok(());
    }

    let mut options = Options::parse(args)?;
    let registry = match options.take("--schema") {
        Some(path) => Registry::from_slice(&read_file(Path::new(&path))?)?,
        None => Registry::builtin()?,
    };
    match command.to_str() {
        Some("schema") => {
            let primitive = options.take("--primitive");
            options.finish()?;
            if let Some(primitive) = primitive {
                print_json(
                    &registry.primitive(primitive.to_str().context("invalid primitive ID")?)?,
                )
            } else {
                print_json(registry.document())
            }
        }
        Some("list") => {
            let directory = options.path("--dir")?;
            options.finish()?;
            let mut mappings = Vec::new();
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && entry.file_name().to_string_lossy().ends_with(".map.json")
                {
                    let path = entry.path();
                    let mapping = load_mapping(&path, &registry)?;
                    mappings.push(serde_json::json!({
                        "file": entry.file_name(),
                        "primitive": mapping.primitive, "provider": mapping.provider, "model": mapping.model,
                        "revision": mapping.revision, "status": mapping.status
                    }));
                }
            }
            mappings.sort_by(|a, b| a["file"].as_str().cmp(&b["file"].as_str()));
            print_json(&mappings)
        }
        Some("validate" | "preview" | "apply" | "approve") => {
            let file = options.path("--file")?;
            let mut mapping = load_mapping(&file, &registry)?;
            match command.to_str() {
                Some("validate") => {
                    options.finish()?;
                    print_json(&serde_json::json!({"valid": true, "status": mapping.status}))
                }
                Some("preview" | "apply") => {
                    let direction = match options.string("--direction")?.as_str() {
                        "request" => Direction::Request,
                        "response" => Direction::Response,
                        _ => bail!("direction must be request or response"),
                    };
                    let input = options.path("--input")?;
                    let identity = if command == "apply" {
                        Some((options.string("--provider")?, options.string("--model")?))
                    } else {
                        None
                    };
                    options.finish()?;
                    let sample = read_json(&input)?;
                    let output = match identity {
                        Some((provider, model)) => {
                            mapping.apply(&registry, &provider, &model, direction, &sample)?
                        }
                        None => mapping.preview(&registry, direction, &sample)?,
                    };
                    print_json(&output)
                }
                _ => {
                    let request = options.path("--request")?;
                    let response = options.path("--response")?;
                    options.finish()?;
                    mapping
                        .preview(&registry, Direction::Request, &read_json(&request)?)
                        .context("request sample failed")?;
                    mapping
                        .preview(&registry, Direction::Response, &read_json(&response)?)
                        .context("response sample failed")?;
                    mapping.status = Status::Approved;
                    mapping.validate(&registry)?;
                    print_json(&mapping)
                }
            }
        }
        Some("draft") => {
            let primitive = options.string("--primitive")?;
            let provider = options.string("--provider")?;
            let model = options.string("--model")?;
            let revision = options
                .take("--revision")
                .unwrap_or_else(|| "v1".into())
                .into_string()
                .map_err(|_| anyhow::anyhow!("invalid revision ID"))?;
            let source = match (options.take("--context"), options.take("--url")) {
                (Some(path), None) => draft::Source::File(path.into()),
                (None, Some(url)) => draft::Source::Url(
                    url.into_string()
                        .map_err(|_| anyhow::anyhow!("invalid URL"))?,
                ),
                _ => bail!("supply exactly one of --context or --url"),
            };
            options.finish()?;
            let mapping = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(draft::create(
                    &registry, primitive, provider, model, revision, source,
                ))?;
            print_json(&mapping)
        }
        _ => bail!("unknown mapping command. Use tokencraft mapping --help"),
    }
}

fn print_json(value: &impl Serialize) -> Result<()> {
    serde_json::to_writer_pretty(io::stdout().lock(), value)?;
    println!();
    Ok(())
}

fn load_mapping(path: &Path, registry: &Registry) -> Result<Mapping> {
    let mapping =
        Mapping::parse(&read_file(path)?).with_context(|| format!("read {}", path.display()))?;
    mapping
        .validate(registry)
        .with_context(|| format!("validate {}", path.display()))?;
    Ok(mapping)
}

fn read_json(path: &Path) -> Result<Value> {
    let bytes = if path == Path::new("-") {
        let mut data = Vec::new();
        io::stdin()
            .lock()
            .take(MAX_FILE + 1)
            .read_to_end(&mut data)?;
        ensure!(data.len() as u64 <= MAX_FILE, "input exceeds 256 KiB");
        data
    } else {
        read_file(path)?
    };
    serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    File::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .take(MAX_FILE + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() as u64 <= MAX_FILE, "file exceeds 256 KiB");
    Ok(data)
}

struct Options(BTreeMap<String, OsString>);

impl Options {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Self> {
        let mut values = BTreeMap::new();
        while let Some(option) = args.next() {
            let name = option
                .into_string()
                .map_err(|_| anyhow::anyhow!("invalid option"))?;
            ensure!(
                name.starts_with("--") && name.len() > 2,
                "expected a named option"
            );
            let value = args
                .next()
                .with_context(|| format!("missing value for {name}"))?;
            ensure!(
                values.insert(name.clone(), value).is_none(),
                "repeated option {name}"
            );
        }
        Ok(Self(values))
    }

    fn take(&mut self, name: &str) -> Option<OsString> {
        self.0.remove(name)
    }

    fn path(&mut self, name: &str) -> Result<PathBuf> {
        self.take(name)
            .map(PathBuf::from)
            .with_context(|| format!("set {name}"))
    }

    fn string(&mut self, name: &str) -> Result<String> {
        self.take(name)
            .and_then(|value| value.into_string().ok())
            .with_context(|| format!("set {name}"))
    }

    fn finish(self) -> Result<()> {
        if let Some(option) = self.0.keys().next() {
            bail!("unknown option {option}");
        }
        Ok(())
    }
}
