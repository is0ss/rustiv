use crate::*;

use crate::error::{ApiError, Kind::OAuth};
use crate::utils::{UnwrapDef, crypto};
use crate::aapi::endpoint;

use serde::{ser::Serialize, de::DeserializeOwned};
use serde_json::Value;
use thin_str::ThinStr;
use std::io;

// taken from the Android app, don't worry about it.
// (latest version can be found using GET https://app-api.pixiv.net/v1/application-info/android)
const CLIENT_ID:      &str = "MOBrBDS8blbauoSck0ZfDbtuzpyT";
const CLIENT_SECRET:  &str = "lsACyCD94FhDUtGTXi3QzcFE2uU1hqtDaKeqrdwj";
const AUTH_TOKEN_URL: &str = "https://oauth.secure.pixiv.net/auth/token";
const REDIRECT_URI:   &str = endpoint!("/web", "/v1/users/auth/pixiv/callback");

pub fn authenticate<T: DeserializeOwned>(client: &HttpClient) -> Result<T> {
    let code_verifier = crypto::random_bytes_b64(32);
    let code_challenge = crypto::s256(&code_verifier);

    let login_url = format!(
        concat!(
            endpoint!("/web", "/v1/login"), // LOGIN_URL
            "?code_challenge={}&code_challenge_method=S256&client=pixiv-android"
        ), code_challenge
    );

    open::that(&login_url)
        .expect("open login url");

    let mut code = String::new();
    io::stdin().read_line(&mut code)?;

    auth_request(client, &[
        ("client_id",      CLIENT_ID),
        ("client_secret",  CLIENT_SECRET),
        ("redirect_uri",   REDIRECT_URI),
        ("code",           code.trim()),
        ("code_verifier",  &code_verifier),
        ("grant_type",     "authorization_code"),
        ("include_policy", "true"),
    ])
}

pub fn refresh<T: DeserializeOwned>(client: &HttpClient, refresh_token: &str) -> Result<T>
{
    auth_request(client, &[
        ("client_id",      CLIENT_ID),
        ("client_secret",  CLIENT_SECRET),
        ("grant_type",     "refresh_token"),
        ("refresh_token",  refresh_token),
        ("get_secure_url", "1"),
    ])
}

fn auth_request<T: ?Sized, V>(client: &HttpClient, params: &T) -> Result<V>
    where
        T: Serialize,
        V: DeserializeOwned,
{
    let json = client.post(AUTH_TOKEN_URL)
        .form(params)
        .send()?
        .json()?;

    check_err(&json)?;
    serde_json::from_value(json).map_err(PixivError::from)
}

#[inline]
fn check_err(v: &Value) -> Result<()> {
    if v["has_error"].unwrap_def() {
        let system = &v["errors"]["system"];
        let msg: ThinStr = system["message"].unwrap_def();
        let code: u16 = system["code"].unwrap_def();

        return Err(
            PixivError::API(ApiError {
                kind: OAuth,
                code: code.into(),
                msg,
            })
        )
    }

    Ok(())
}
