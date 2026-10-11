use aitestloop_refactor_factory::DummyFactory;

#[test]
fn factory_refactoring() {
    // Test successful creation
    let dummy = DummyFactory::create(true).expect("Failed to create Dummy");
    assert_eq!(dummy.value(), 1);
    
    // Test failed creation
    let dummy = DummyFactory::create(false);
    assert!(dummy.is_none());
}
