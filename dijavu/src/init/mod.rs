mod container;
pub use self::container::InitAppContainer;

mod initializable;
pub use self::initializable::{Initializable, StartValue, Value};

/// A type that can be retrieved or constructed during initialization
///
/// `InitInjectable` is the initialization-phase counterpart to [`Injectable`](crate::Injectable).
/// It allows types to define how they are accessed or constructed from an
/// [`InitAppContainer`].
pub trait InitInjectable: Sized + 'static {
    /// Init-time injection error type
    type InitError;
    /// Init-time injected value
    ///
    /// This type may borrow mutably from a [`InitAppContainer`] with lifetime `'a`.
    type Init<'a>;

    /// Retrieves or constructs the initialization value from the container.
    fn get_init(container: &mut InitAppContainer) -> Result<Self::Init<'_>, Self::InitError>;
}
