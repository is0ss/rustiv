use crate::{HttpClient, PixivError, aapi::endpoint};

use std::io::{self, Write};

use reqwest::IntoUrl;

pub trait IntoStr {
    fn into_str(self) -> &'static str;
}

impl<T: Into<&'static str>> IntoStr for T {
    fn into_str(self) -> &'static str {
        self.into()
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