# Idiomatic Rust Guidance Document

## 1. Introduction

follow these guidelines

## 2. Code Organization and Module Structure

### Modularity

- **DO:** Split your code into modules and crates logically.
- **DO:** Use modules to group related functionality.
- **DON'T:** Create monolithic files that are hard to navigate.

### File Organization

- **DO:** Follow the standard convention where each module corresponds to a file (or a folder with a `mod.rs` file).
- **DON'T:** Mix module definitions and implementations arbitrarily.

### Visibility

- **DO:** Limit the scope of functions and data structures by using the `pub` keyword only when necessary.
- **DON'T:** Expose internal APIs that could be misused by external code.

## 3. Ownership, Borrowing, and Lifetimes

### Ownership Principles

- **DO:** Embrace Rust’s ownership rules and transfer ownership when necessary.
- **DON'T:** Overuse cloning; instead, consider borrowing to minimize unnecessary copies.

### Borrowing

- **DO:** Use immutable references (`&T`) by default.
- **DO:** Use mutable references (`&mut T`) only when modifications are needed.
- **DON'T:** Hold references longer than necessary.

### Lifetimes

- **DO:** Understand and annotate lifetimes when required.
- **DON'T:** Overcomplicate lifetimes; allow the compiler to infer lifetimes when possible.

## 4. Error Handling

### Result and Option

- **DO:** Use `Result<T, E>` for functions that can fail.
- **DO:** Use `Option<T>` when a value might be absent.
- **DON'T:** Panic in library code; propagate errors using the `?` operator.

### Custom Error Types

- **DO:** Define custom error types as needed.
- **DON'T:** Use generic errors without context.

### Error Context

- **DO:** Use libraries like [anyhow](https://docs.rs/anyhow) or [thiserror](https://docs.rs/thiserror) to add context to errors.
- **DON'T:** Ignore error context that could help diagnose issues.

## 5. Pattern Matching and Control Flow

### Exhaustive Matching

- **DO:** Use pattern matching (`match`) to destructure enums and other types.
- **DON'T:** Rely solely on `if` statements when `match` can provide exhaustive handling.

### If Let and While Let

- **DO:** Use `if let` and `while let` for concise handling of patterns.
- **DON'T:** Overuse these constructs where a full `match` might be clearer.

### Guard Clauses

- **DO:** Use pattern guards in match arms to combine conditions with patterns.
- **DON'T:** Write overly complex guards that reduce readability.

## 6. Traits, Generics, and Abstractions

### Traits

- **DO:** Define traits to abstract shared behavior.
- **DON'T:** Overcomplicate trait bounds unnecessarily.

### Generics

- **DO:** Use generics to write reusable code.
- **DON'T:** Sacrifice clarity for generality—prefer concrete types when it enhances understanding.

### Trait Objects

- **DO:** Use dynamic dispatch (`Box<dyn Trait>`) when necessary.
- **DON'T:** Use dynamic dispatch without considering performance implications.

## 7. Concurrency and Parallelism

### Thread Safety

- **DO:** Use Rust’s concurrency primitives (e.g., `Mutex`, `RwLock`, channels).
- **DON'T:** Share mutable state between threads without proper synchronization.

### Fearless Concurrency

- **DO:** Exploit Rust’s guarantees to write concurrent code without data races.
- **DON'T:** Assume that concurrency is inherently error-free—always consider edge cases.

### Async/Await

- **DO:** Embrace asynchronous programming using the `async`/`await` syntax.
- **DON'T:** Block the async runtime with long-running synchronous operations.

## 8. Testing, Documentation, and Community Tools

### Unit Tests

- **DO:** Write unit tests for your functions using Rust’s built-in test framework.
- **DON'T:** Neglect testing or rely solely on manual testing.

### Documentation

- **DO:** Document public APIs with Rustdoc comments (`///`).
- **DON'T:** Leave public functions undocumented.

### Benchmarks and Integration Tests

- **DO:** Use benchmarks to measure performance-critical code.
- **DON'T:** Ignore integration testing for complex systems.

### Linting and Formatting

- **DO:** Use tools like `rustfmt` and `clippy` for consistent formatting and linting.
- **DON'T:** Skip linting checks, as they can catch common pitfalls.

## 9. Code Style and Idioms

### Naming Conventions

- **DO:** Use `snake_case` for variables, functions, and modules.
- **DO:** Use `CamelCase` for types, traits, and enums.
- **DON'T:** Mix naming conventions within the same project.

### Error Messages

- **DO:** Write clear and concise error messages.
- **DON'T:** Use vague error messages that hinder debugging.

### Immutability

- **DO:** Prefer immutability by default.
- **DON'T:** Make variables mutable unless absolutely necessary.

### Iterators and Functional Style

- **DO:** Leverage iterators, closures, and combinators (`map`, `filter`, `fold`).
- **DON'T:** Write imperative loops when a functional style is more expressive.

### Documentation Comments

- **DO:** Include doc comments for modules, functions, and data structures.
- **DON'T:** Assume that code is self-documenting—provide context and examples where necessary.

## 10. Rust Documentation and Comments

### Writing Rust Documentation

- **DO:** Use Rust's built-in documentation system with triple-slash comments (`///`) to create clear, user-friendly documentation via rustdoc.
- **DO:** Clearly describe the purpose of functions, modules, types, and their parameters, return values, and error conditions.
- **DO:** Provide examples and usage scenarios within your documentation. Leverage doc-tests to ensure examples remain accurate and serve as living documentation.
- **DO:** Utilize markdown formatting in your doc comments to enhance readability—include code blocks, lists, and hyperlinks as needed.
- **DON'T:** Rely solely on inline comments for public-facing documentation. Avoid redundant or overly verbose explanations that repeat what the code already conveys.
- **DON'T:** Use special characters in comments and output. 

### Crafting Effective Comments

- **DO:** Write inline comments to explain non-obvious logic, design decisions, or complex code paths that may not be immediately clear from the code itself.
- **DO:** Keep comments concise and focused, ensuring they add value without cluttering the code.
- **DO:** Update comments alongside code changes to maintain accuracy.
- **DON'T:** Over-comment trivial or self-explanatory code. Comments should enhance understanding, not state the obvious.
- **DON'T:** Leave outdated or misleading comments that can cause confusion for future maintainers.

