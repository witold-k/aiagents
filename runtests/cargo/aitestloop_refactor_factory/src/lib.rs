pub struct Dummy;

impl Dummy {
    fn new() -> Self { Self }

    pub fn value(&self) -> i32 { 1 }
}

pub struct DummyFactory;

impl DummyFactory {
    pub fn create(allowed: bool) -> Option<Dummy> {
        allowed.then(Dummy::new)
    }
}
