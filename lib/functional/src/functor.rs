pub trait Functor<'a, A> {
    type Unwrapped;
    type Wrapped<B: 'a>: Functor<'a, B>;

    fn map<F, B>(self, map_fn: F) -> Self::Wrapped<B>
    where
        B: 'a,
        F: FnOnce(A) -> B + 'a;

    fn flat_map<F, B>(self, map_fn: F) -> Self::Wrapped<B>
    where
        B: 'a,
        F: FnOnce(A) -> Self::Wrapped<B> + 'a;
}
