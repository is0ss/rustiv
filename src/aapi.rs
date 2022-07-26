use crate::*;

use crate::error::{ApiError, Kind::AAPI};
use crate::utils::{IntoStr, UnwrapRef};

use std::{io::Write, time::Duration};
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use reqwest::{blocking::Response, IntoUrl, Method};
use thin_str::ThinStr;
use serde_json::Value;

macro_rules! endpoint {
    ($($l:literal),*) => {
        concat!("https://app-api.pixiv.net", $($l,)*)
    };
}
pub(crate) use endpoint;

const USER_AGENT: &str = "User-Agent: PixivAndroidApp/5.0.234 (Android 11; Pixel 5)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PixivId(u64);

impl From<&str> for PixivId {
    fn from(value: &str) -> Self {
        Self(value.parse().unwrap_or(0))
    }
}

macro_rules! id_from_ints {
    ($($type:ty,)+) => {$(
        impl From<$type> for PixivId {
            fn from(value: $type) -> Self {
                Self(value as u64)
            }
        }
    )+}
}

id_from_ints! {
    u8,    i8,
    u16,   i16,
    u32,   i32,
    u64,   i64,
    usize, isize,
}

struct PixivRequestBuilder<'a, T: ?Sized, U: IntoUrl> {
    pixiv: &'a PixivClient,

    method: Method,
    query: &'a T,
    url: U,
}

pub struct PixivResponse<T> {
    res: Result<Response>,
    _p: PhantomData<T>,
}

pub struct Unsupported;

impl<'a, T: ?Sized, U: IntoUrl> PixivRequestBuilder<'a, T, U>
where
    T: Serialize,
{
    fn new(pixiv: &'a PixivClient, method: Method, url: U, query: &'a T) -> Self {
        Self { pixiv, method, query, url }
    }

    fn send<R>(self) -> PixivResponse<R> {
        PixivResponse::from(
            self.pixiv.client()
                .request(self.method, self.url)
                .bearer_auth(self.pixiv.access_token())
                .query(self.query)
                .send()
        )
    }
}

impl<T> PixivResponse<T> {
    pub fn raw(self) -> Result<String> {
        self.res?.text().map_err(PixivError::from)
    }

    fn deserialize<D: DeserializeOwned>(self) -> Result<D> {
        self.res?.json().map_err(PixivError::from)
    }

    pub fn value(self) -> Result<Value> {
        self.deserialize()
    }

    #[inline]
    fn parse_response(mut res: reqwest::Result<Response>) -> Result<Response> {
        if let Ok(ref mut res) = res && res.status().is_client_error() {
            let mut buf: Vec<u8> = vec![];
            res.copy_to(&mut buf)?;

            let err = &serde_json::from_slice::<Value>(&buf)?["error"];

            let user_message = err["user_message"].unwrap_ref();
            let message = err["message"].unwrap_ref();
            let reason = err["reason"].unwrap_ref();

            return Err(
                PixivError::API(ApiError {
                    kind: AAPI,
                    code: res.status().as_u16().into(),
                    msg:  ThinStr::new(format!("{reason}{user_message}{message}").as_str())
                })
            )
        }

        res.map_err(PixivError::from)
    }
}

impl<T: DeserializeOwned> PixivResponse<T> {
    pub fn deserialize_into(self) -> Result<T> {
        self.deserialize()
    }
}

impl<T> From<reqwest::Result<Response>> for PixivResponse<T> {
    fn from(res: reqwest::Result<Response>) -> Self {
        Self { res: Self::parse_response(res), _p: PhantomData }
    }
}

#[derive(Debug)]
pub struct PixivClient {
    client: Box<HttpClient>,
    auth: AuthInfo,
}

#[derive(Deserialize, Debug)]
struct AuthInfo {
    user:  PixivClientUser,

    access_token:  ThinStr,
    refresh_token: ThinStr,
    expires_in:        u16,
}

impl PixivClient {
    #[inline]
    fn client(&self) -> &HttpClient {
        &self.client
    }

    #[inline]
    pub fn user(&self) -> &PixivClientUser {
        &self.auth.user
    }

    #[inline]
    pub fn access_token(&self) -> &str {
        &self.auth.access_token
    }

    #[inline]
    pub fn refresh_token(&self) -> &str {
        &self.auth.refresh_token
    }

    #[inline]
    pub fn expires_in(&self) -> Duration {
        Duration::from_secs(self.auth.expires_in as u64)
    }
}

#[derive(Deserialize, Debug)]
struct UserInfo {
    #[serde(deserialize_with = "crate::utils::de::pid")]
    id:      PixivId,
    name:    ThinStr,
    account: ThinStr,
}

#[derive(Deserialize, Debug)]
pub struct PixivClientUser {
    #[serde(flatten)]
    user_info: UserInfo,

    #[serde(rename = "mail_address")]
    mail_addr: ThinStr,
    #[serde(rename = "profile_image_urls", deserialize_with = "crate::utils::de::pfps")]
    pfp_urls: [ThinStr; 3], // 16 x 16, 50 x 50, 170 x 170
    #[serde(rename = "is_mail_authorized")]
    m_authed:     bool,
    #[serde(rename = "is_premium")]
    premium:      bool,

    x_restrict:     u8,
}

impl PixivClientUser {
    #[inline]
    pub fn id(&self) -> PixivId {
        self.user_info.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.user_info.name
    }

    #[inline]
    pub fn account(&self) -> &str {
        &self.user_info.account
    }

    #[inline]
    pub fn mail_addr(&self) -> &str {
        &self.mail_addr
    }

    #[inline]
    pub fn pfp_urls(&self) -> &[ThinStr; 3] {
        &self.pfp_urls
    }

    #[inline]
    pub fn x_restrict(&self) -> u8 {
        self.x_restrict
    }

    #[inline]
    pub fn is_mail_authed(&self) -> bool {
        self.m_authed
    }

    #[inline]
    pub fn is_premium(&self) -> bool {
        self.premium
    }
}

impl PixivClient {
    #[inline]
    fn http_client(builder: HttpBuilder) -> reqwest::Result<HttpClient> {
        builder
            .user_agent(USER_AGENT)
            .referer(false)
            .build()
    }

    #[inline]
    fn new(client: HttpClient, auth: AuthInfo) -> Result<Self> {
        Ok(Self { client: Box::new(client), auth })
    }

    pub fn authenticate(builder: HttpBuilder) -> Result<Self> {
        let client = Self::http_client(builder)?;
        let auth = oauth::authenticate(&client)?;

        Self::new(client, auth)
    }

    pub fn refresh_auth(builder: HttpBuilder, refresh_token: &str) -> Result<Self> {
        let client = Self::http_client(builder)?;
        let auth = oauth::refresh(&client, refresh_token)?;

        Self::new(client, auth)
    }

    pub fn refresh(&mut self) -> Result<()> {
        Ok(
            self.auth = oauth::refresh(self.client(), self.refresh_token())?
        )
    }

    pub fn download<U, W: ?Sized>(&self, url: U, writer: &mut W) -> Result<u64>
    where
        U: IntoUrl,
        W: Write,
    {
        pixiv_download(self.client(), url, writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn it_authenticates() {
        dbg!(
            PixivClient::authenticate(HttpBuilder::new()).unwrap()
        );
    }

    fn refresh(refresh_token: &str) -> PixivClient {
        PixivClient::refresh_auth(HttpBuilder::new(), refresh_token).unwrap()
    }

    #[test]
    fn it_refreshes_auth() {
        dbg!(
            refresh(env!("REFRESH_TOKEN"))
        );
    }

    #[test]
    #[should_panic]
    fn fail_refresh_auth() {
        refresh("");
    }
}
