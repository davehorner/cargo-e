// tests/test_funny_docs.rs

#[test]
fn integration_test_funny_docas() {
    use cargo_e::a_funny_docs::guide; // Assuming your crate is named `cargo_e`
    use cargo_e::a_funny_docs::ATrait;
    use cargo_e::funny_macro;

    // Test the constant.
    let _ = guide::A_CONST;

    // Test the function.
    guide::a_function();

    // Test the struct.
    let _s = guide::AStruct;

    // Test the enum.
    match guide::AEnum::AVariant {
        guide::AEnum::AVariant => {}
    }

    // Test the trait.
    struct TestDummy;
    impl guide::ATrait for TestDummy {
        fn do_joke(&self) -> String {
            "Integration test joke!".to_string()
        }
    }
    let dummy = TestDummy;
    assert_eq!(dummy.do_joke(), "Integration test joke!");

    // Test the type alias.
    let _alias: guide::AType = ();

    // Test the dynamic type.
    let _dynamic: cargo_e::a_funny_docs::ACrazyDuck = cargo_e::a_funny_docs::ACrazyDuck;

    // Test the macro.
    funny_macro!();
}

// tests/test_funny_docs.rs

#[test]
fn integration_test_funny_docs() {
    use cargo_e::a_funny_docs::guide; // Assuming your crate is named `cargo_e`
                                      // Test the constant.
    let _ = guide::A_CONST;

    // Test the function.
    guide::a_function();

    // Test the struct.
    let _s = guide::AStruct;

    // Test the enum.
    match guide::AEnum::AVariant {
        guide::AEnum::AVariant => {}
    }

    // Test the trait.
    struct TestDummy;
    impl guide::ATrait for TestDummy {
        fn do_joke(&self) -> String {
            "Integration test joke!".to_string()
        }
    }
    let _dummy = TestDummy;
    //assert_eq!(dummy.do_joke(), "Integration test joke!");

    // Test the type alias.
    let _alias: guide::AType = ();

    // Test the dynamic type.
    let _dynamic: cargo_e::a_funny_docs::ACrazyDuck = cargo_e::a_funny_docs::ACrazyDuck;

    // Test the macro.
    cargo_e::funny_macro!();
}
