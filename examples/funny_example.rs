// examples/funny_example.rs

fn main() {
    use cargo_e::a_funny_docs::guide;
    use cargo_e::a_funny_docs::ATrait;
    println!("Testing humorous docs...");

    /// Demonstrates the art of appeasing the compiler:
    ///
    /// Instead of just calling `guide::A_CONST` (which would trigger a "path statement with no effect"
    /// warning), we bind it to `_` even though it's just `()`. Yes, it's like giving a participation
    /// trophy to something that did absolutely nothing. We also silence the Clippy lint about binding a
    /// unit value, because sometimes, even nothing deserves a little recognition.
    ///
    /// # Note
    ///
    /// This function is never used, but it lives on in our docs as a shining example of minimalist code.
    #[allow(dead_code, clippy::let_unit_value)]
    fn use_a_const() {
        let _ = guide::A_CONST;
    }

    // Function.
    guide::a_function();

    // Struct.
    let _instance = guide::AStruct;

    // Enum.
    match guide::AEnum::AVariant {
        guide::AEnum::AVariant => println!("Enum variant matched!"),
    }

    // Trait implementation.
    struct ExampleDummy;
    impl guide::ATrait for ExampleDummy {
        fn do_joke(&self) -> String {
            "Example dummy joke!".into()
        }
    }
    let ex = ExampleDummy;
    println!("{}", ex.do_joke());

    // Type alias.
    let _alias: guide::AType = ();

    // Dynamic type from macro (ACrazyDuck).
    let _funny_instance: cargo_e::a_funny_docs::ACrazyDuck = cargo_e::a_funny_docs::ACrazyDuck;

    // Call the funny macro.
    cargo_e::funny_macro!();
}
