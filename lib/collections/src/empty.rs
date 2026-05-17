pub trait Empty: Default {
    fn empty() -> Self {
        Self::default()
    }
}

impl<T: Default> Empty for T {}
