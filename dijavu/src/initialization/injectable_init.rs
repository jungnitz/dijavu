use crate::data::DataEntry;
use crate::{InitInjector, Injectable, InjectableDataKey, Restricted};

/// Provides access to an [`Injectable`]'s initialization data and a corresponding [`InitInjector`].
pub struct InjectableInit<'a, I: Injectable>(InjectableInitInner<'a, I>);

enum InjectableInitInner<'a, I: Injectable> {
    References(&'a mut InitInjector, &'a mut I::Data),
    Injector(&'a mut InitInjector),
}

impl<'a, I> InjectableInit<'a, I>
where
    I: Injectable,
{
    pub fn new(data: &'a mut I::Data, injector: &'a mut InitInjector) -> Self {
        Self(InjectableInitInner::References(injector, data))
    }

    fn new_assert_initialized(injector: &'a mut InitInjector) -> Self {
        Self(InjectableInitInner::Injector(injector))
    }

    pub(super) async fn init(injector: &'a mut InitInjector) -> Result<Self, I::Error> {
        match injector.data_mut().entry::<InjectableDataKey<I>>() {
            DataEntry::Occupied(entry) => {
                if entry.get().is_some() {
                    return Ok(Self::new_assert_initialized(injector));
                } else {
                    // this ensures that `I::init` is called exactly once
                    // panicking here is fine, because it is actually relatively difficult to
                    // encounter this condition.
                    // usually, the compiler will yell at you because of recursive futures --
                    // unless you box them, which most likely nobody will ever do for their
                    // `init`-function
                    panic!("recursive initialization");
                }
            }
            DataEntry::Vacant(entry) => entry.insert(None),
        };
        let data = I::new_init_data(injector, Restricted::new()).await?;

        let Some(value) = injector.data_mut().get_mut::<InjectableDataKey<I>>() else {
            unreachable!("value was inserted before and is not removed before build");
        };
        assert!(
            value.is_none(),
            "only the `init` call that inserted `None` should populate its value"
        );
        *value = Some(data);

        // make sure that `I` is always built when initialized
        injector.enqueue_assert_initialization::<I>();
        Ok(Self::new_assert_initialized(injector))
    }

    pub fn injector_mut(&mut self) -> &mut InitInjector {
        self.as_owned().into_injector()
    }

    pub fn data_mut(&mut self) -> &mut I::Data {
        self.as_owned().into_data()
    }

    pub fn data(&self) -> &I::Data {
        match &self.0 {
            InjectableInitInner::References(_, data) => data,
            InjectableInitInner::Injector(injector) => injector
                .data()
                .get::<InjectableDataKey<I>>()
                .unwrap()
                .as_ref()
                .unwrap(),
        }
    }

    fn as_owned(&mut self) -> InjectableInit<'_, I> {
        match &mut self.0 {
            InjectableInitInner::References(inj, data) => {
                InjectableInit(InjectableInitInner::References(inj, &mut **data))
            }
            InjectableInitInner::Injector(inj) => {
                InjectableInit(InjectableInitInner::Injector(inj))
            }
        }
    }

    pub fn into_injector(self) -> &'a mut InitInjector {
        match self.0 {
            InjectableInitInner::References(injector, _) => injector,
            InjectableInitInner::Injector(injector) => injector,
        }
    }

    pub fn into_data(self) -> &'a mut I::Data {
        match self.0 {
            InjectableInitInner::References(_, data) => data,
            InjectableInitInner::Injector(injector) => injector
                .data_mut()
                .get_mut::<InjectableDataKey<I>>()
                .unwrap()
                .as_mut()
                .unwrap(),
        }
    }
}
