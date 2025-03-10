// examples/funny_example.rs

fn main() {
    use cargo_e::a_funny_docs::guide;
    use cargo_e::a_funny_docs::ATrait;
    println!("Testing humorous docs...");

    // Constant.
    let _ = guide::A_CONST;

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
