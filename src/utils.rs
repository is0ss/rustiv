use crate::{HttpClient, PixivError, aapi::endpoint};

use std::io::{self, Write};

use serde_json::Value;
use thin_str::ThinStr;
use reqwest::IntoUrl;

pub trait IntoStr {
    fn into_str(self) -> &'static str;
}

impl<T: Into<&'static str>> IntoStr for T {
    fn into_str(self) -> &'static str {
        self.into()
    }
}

pub trait UnwrapDef<T> {
    fn unwrap_def(&self) -> T;
}

impl UnwrapDef<ThinStr> for Value {
    fn unwrap_def(&self) -> ThinStr {
        ThinStr::new(self.as_str().unwrap_or_default())
    }
}

impl UnwrapDef<bool> for Value {
    fn unwrap_def(&self) -> bool {
        self.as_bool().unwrap_or_default()
    }
}

impl UnwrapDef<u16> for Value {
    fn unwrap_def(&self) -> u16 {
        self.as_u64().unwrap_or_default() as u16
    }
}

pub fn pixiv_download<W: ?Sized>(client: &HttpClient, url: impl IntoUrl, writer: &mut W) -> crate::Result<u64>
where
    W: Write,
{
    let bytes = client.get(url)
        .header("Referer", endpoint!()) // Referer is App-API base url
        .send()?
        .error_for_status()?
        .bytes()?;

    io::copy(&mut bytes.as_ref(), writer).map_err(PixivError::from)
}

pub mod crypto {
    use rand::{thread_rng, Rng};
    use sha2::{Digest, Sha256};

    pub fn random_bytes_b64(len: u16) -> String {
        let random_bytes: Vec<u8> = (0..len).map(|_| thread_rng().gen::<u8>()).collect();
        base64::encode_config(&random_bytes, base64::URL_SAFE_NO_PAD)
    }

    pub fn s256(data: &str) -> String {
        let digest = Sha256::digest(data.as_bytes());
        base64::encode_config(&digest, base64::URL_SAFE_NO_PAD)
    }
}