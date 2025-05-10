use std::sync::Arc;
use serde_json::Value;

type LayoutResolver = Box<dyn Fn(String) -> String + Send + Sync>;
type SharedPropsFn = dyn Fn() -> Value + Send + Sync;

struct Inner {
    version: Option<String>,
    layout: LayoutResolver,
    shared_props: Vec<Arc<SharedPropsFn>>
}

#[derive(Clone)]
pub struct InertiaConfig {
    inner: Arc<Inner>,
}

impl InertiaConfig {
    /// Constructs a new InertiaConfig object.
    ///
    /// `layout` provides information about how to render the initial
    /// page load. See the [crate::vite] module for an implementation
    /// of this for vite.
    pub fn new(version: Option<String>, layout: LayoutResolver,  shared_props: Vec<Arc<SharedPropsFn>>) -> InertiaConfig {
        let inner = Inner { version, layout, shared_props };
        InertiaConfig {
            inner: Arc::new(inner),
        }
    }

    /// Returns a cloned optional version string.
    pub fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    /// Returns a reference to the layout function.
    pub fn layout(&self) -> &LayoutResolver {
        &self.inner.layout
    }

    pub fn shared_props(&self) -> &Vec<Arc<SharedPropsFn>> {
        &self.inner.shared_props
    }
}
