pub trait Message
{
    #[cfg(feature = "std")]
    fn print_fields(&self);
}