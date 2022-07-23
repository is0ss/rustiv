macro_rules! endpoint {
    ($($l:literal),*) => {
        concat!("https://app-api.pixiv.net", $($l,)*)
    };
}
pub(crate) use endpoint;