use anyhow::anyhow;
use serde_json::Value;

use crate::cfg::CliConfig;

fn url(cfg: &CliConfig, path: &str) -> String {
    format!("{}{}", cfg.rpc_url.trim_end_matches('/'), path)
}

pub fn get(cfg: &CliConfig, path: &str) -> anyhow::Result<Value> {
    match ureq::get(&url(cfg, path)).call() {
        Ok(resp) => Ok(resp.into_json()?),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(anyhow!("http {code}: {body}"))
        }
        Err(e) => Err(anyhow!("request failed: {e}")),
    }
}

pub fn post(cfg: &CliConfig, path: &str, body: Value) -> anyhow::Result<Value> {
    match ureq::post(&url(cfg, path)).send_json(body) {
        Ok(resp) => Ok(resp.into_json()?),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(anyhow!("http {code}: {body}"))
        }
        Err(e) => Err(anyhow!("request failed: {e}")),
    }
}
