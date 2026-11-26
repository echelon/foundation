#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessRole {
    Combined,
    Api,
    Worker,
}

impl ProcessRole {
    pub(crate) fn load() -> anyhow::Result<Self> {
        match std::env::var("TOKENCRAFT_PROCESS_ROLE").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("combined") => Ok(Self::Combined),
            Ok("api") => Ok(Self::Api),
            Ok("worker") => Ok(Self::Worker),
            _ => anyhow::bail!("set TOKENCRAFT_PROCESS_ROLE to combined, api, or worker"),
        }
    }

    pub(super) fn serves_api(self) -> bool {
        matches!(self, Self::Api | Self::Combined)
    }

    pub(super) fn runs_workers(self) -> bool {
        matches!(self, Self::Worker | Self::Combined)
    }
}
