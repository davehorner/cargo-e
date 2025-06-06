# e_obs

`e_obs` is a Rust-based utility designed to interact with OBS (Open Broadcaster Software) via its WebSocket API. It provides functionality for executing commands, capturing screenshots, setting wallpapers, and creating animated GIFs, making it a versatile tool for automating OBS workflows.

## Features

- **Command Execution**: Execute multiple commands sequentially using the `--cmd` argument.
- **Screenshot Management**: Capture screenshots during command execution and organize them into separate directories.
- **Wallpaper Setting**: Automatically set a random screenshot from the last third of the previous command's screenshots as the desktop wallpaper.
- **GIF Creation**: Generate animated GIFs from screenshots for each executed command.
- **OBS Integration**: Seamlessly interact with OBS to manage recording directories, capture sources, and more.

## Installation

To use `e_obs`, ensure you have the following prerequisites:

- Rust (latest stable version recommended)
- OBS Studio with the WebSocket plugin enabled

Clone the repository and build the project:

```bash
# Clone the repository
git clone https://github.com/your-repo/e_obs.git

# Navigate to the project directory
cd e_obs

# Build the project
cargo build --release
```

## Usage

Run `e_obs` with the desired arguments:

```bash
./e_obs --cmd "command1" "command2" --num-screenshots 5
```

### Arguments

- `--cmd`: A list of commands to execute sequentially.
- `--num-screenshots`: (Optional) Number of screenshots to capture per command.
- `--version` or `-V`: Display the version of `e_obs`.

## Example Workflow

1. Start OBS Studio and ensure the WebSocket server is running.
2. Execute `e_obs` with the desired commands:

   ```bash
   ./e_obs --cmd "echo Hello" "ls -la" --num-screenshots 3
   ```

3. Screenshots will be saved in directories named `cmd_1`, `cmd_2`, etc.
4. A random screenshot from the last third of the previous command's screenshots will be set as the wallpaper.
5. Animated GIFs will be created for each command's screenshots.

## Development

To contribute to `e_obs`, follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Submit a pull request with a detailed description of your changes.

### Testing

Run the tests to ensure everything is working as expected:

```bash
cargo test
```

## License

`e_obs` is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE-2.0) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- [OBS Studio](https://obsproject.com/) for providing an excellent open-source broadcasting tool.
- [tokio](https://tokio.rs/) for asynchronous programming in Rust.
- [rand](https://crates.io/crates/rand) for random number generation.

## Contact

For questions or support, please open an issue on the [GitHub repository](https://github.com/your-repo/e_obs/issues).
