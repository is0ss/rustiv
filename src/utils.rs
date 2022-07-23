pub trait IntoStr {
    fn into_str(self) -> &'static str;
}

impl<T: Into<&'static str>> IntoStr for T {
    fn into_str(self) -> &'static str {
        self.into()
    }
}