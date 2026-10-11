pub struct Dummy;

impl Dummy {
    pub fn new() -> Self { Self }

    pub fn value(&self) -> i32 { 1 }
}

impl Default for Dummy {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DummyFactory;

impl DummyFactory {
    pub fn create(allowed: bool) -> Option<Dummy> {
        allowed.then(Dummy::new)
    }
}
