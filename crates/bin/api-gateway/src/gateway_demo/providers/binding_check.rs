use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

use crate::gateway_demo::catalog::{Adapter, Catalog};

use super::{load_bindings, pin_catalog};

const HELP: &str = "Usage: api-gateway catalog check-bindings --catalog FILE

Validate the catalog and the gateway's queue and chat binding configuration.
Use TOKENCRAFT_CONNECTORS and TOKENCRAFT_LOCAL_CHAT_BINDINGS for the files
in use, and set the referenced service credentials. No database or provider
connection is made. The JSON report omits endpoints and credentials.
";

#[derive(Serialize)]
struct Report<'a> {
    version: u8,
    catalog_revision: &'a str,
    offers: Vec<OfferBinding<'a>>,
}

#[derive(Serialize)]
struct OfferBinding<'a> {
    offer_id: &'a str,
    model: &'a str,
    adapter: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    binding: Option<&'a str>,
    tenant_keys: bool,
}

pub(in crate::gateway_demo) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let path = match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("--catalog") => args.next().context("set --catalog FILE")?,
        None | Some("--help" | "-h") => {
            print!("{HELP}");
            return Ok(());
        }
        _ => bail!("unknown binding check option. Use --help"),
    };
    ensure!(args.next().is_none(), "supply one --catalog FILE");
    let mut catalog = Catalog::load(Path::new(&path))?;
    let (chat, queue) = load_bindings(&catalog)?;
    pin_catalog(&mut catalog, &chat, &queue)?;

    let offers = catalog
        .offers
        .iter()
        .map(|offer| {
            let name = offer.local_binding.as_deref();
            let tenant_keys = name.is_some_and(|name| match offer.adapter {
                Adapter::ConnectorQueue => queue.for_offer(name).is_some_and(|binding| binding.tenant_keys),
                Adapter::OpenAiCompatibleChat => chat
                    .get(name)
                    .is_some_and(|binding| binding.client.requires_tenant_key()),
                Adapter::Mock => false,
            });
            OfferBinding {
                offer_id: &offer.id,
                model: &offer.model,
                adapter: offer.adapter.as_str(),
                binding: name,
                tenant_keys,
            }
        })
        .collect();

    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(
        &mut stdout,
        &Report { version: 1, catalog_revision: &catalog.revision, offers },
    )?;
    writeln!(stdout)?;
    Ok(())
}
