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
git clone git@github.com:davehorner/cargo-e.git

# Navigate to the project directory
cd cargo-e/addendum/e_obs

# Build the project
cargo build --release
```

## Usage

You need to install obs first.
```
choco obs-studio.install
```
or via the installer.  e_obs uses default paths, if you installed in another location, adjust paths.

## Enabling OBS WebSocket Server

To enable the OBS WebSocket server, follow these steps:

1. Go to **Tools > WebSocket Server Settings**.

2. In the dialog:
   - Check **Enable WebSocket server**.
   - Optionally set a port (default: `4455`) and password (recommended).
   - Check **Enable authentication** if you want to use a password.

3. Click **OK**.

OBS WebSocket is now running on `ws://localhost:4455` by default.

### Importing `e_obs_scene_collection.json`

To use the pre-configured scene collection provided by `e_obs`, follow these steps:

1. Open OBS Studio.
2. Go to **Scene Collection > Import**.
3. Locate and select the `e_obs_scene_collection.json` file included in this project.
4. Click **Open** to import the scene collection.

The imported scene collection will now be available in OBS Studio.

### Arguments

Run `e_obs` with the desired arguments:

```bash
OBS Control Script

Usage: e_obs [OPTIONS]

Options:
  -p, --password <PASSWORD>      OBS WebSocket password (if any)
      --set-text <NAME> <VALUE>  Set text fields: --set-text "name" "value"
      --cmd <CMD>                Commands to run sequentially
      --disable-screenshots      Disable screenshots during recording
  -h, --help                     Print help
```

## Example Workflow

`e_obs` will start OBS Studio automatically if it is not already running. This ensures that your automation workflow can begin without manually launching OBS each time. If OBS is installed in a non-default location, make sure to specify the correct path or adjust your environment variables accordingly.

1. Start OBS Studio, ensure the WebSocket server is running.
2. Configure your profile, scenes, sources.
3. Execute `e_obs` with the desired commands:
   ```bash
   git clone https://github.com/davehorner/cuneus
   cd cuneus
   set CUNEUS_MEDIA="C:\w\demos\tauri\streaming_example_test_video.mp4"
   set CUNEUS_RANDOM=1
   e_obs --cmd "startt -f -fg -g6x7m1 -T 1 -hT -hB -sd 1 -rpf -apc 0x6 -rpc cargo-e -f --run-all 90 --run-at-a-time 50 --cached" --cmd "startt -f -g2x2m1 cargo-e -f --run-all 5 --run-at-a-time 4" --cmd "startt -f -fg -g2x2m1 cargo-e -f --run-all 5 --run-at-a-time 3 --cached"
   ```
4. Screenshots will be saved in directories named `cmd_1`, `cmd_2`, etc.
4. A random screenshot from the last third of the previous command's screenshots will be set as the wallpaper.
5. Animated GIFs will be created for each command's screenshots.

The above runs 3 commands;  each produces an animated gif in
C:\Users\<username>\Videos\e_obs\cuneus_YYYY-MM-DD_mm-hh-ss


![cmd_1_animation](https://github.com/user-attachments/assets/e90a813d-ae31-4615-a8b0-d4f60b979b6d)

https://www.youtube.com/watch?v=5BXStX87Z0o&t=46s

![cmd_2_resized](https://github.com/user-attachments/assets/9ec60b36-2153-4f14-8d27-6b492274f92f)
![cmd_3_resized](https://github.com/user-attachments/assets/26d53c73-db45-46f8-b1de-41158c4974ed)
## Development

To contribute to `e_obs`, follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Submit a pull request with a detailed description of your changes.


## License

`e_obs` is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE-2.0) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- [OBS Studio](https://obsproject.com/) for providing an excellent open-source broadcasting tool.
- [tokio](https://tokio.rs/) for asynchronous programming in Rust.
- [rand](https://crates.io/crates/rand) for random number generation.



## Platform Support

`e_obs` is currently supported on **Windows only**. Compatibility with other operating systems may be added in future updates.  The paths are windows; PRs welcome.

## Contact

For questions or support, please open an issue on the [https://github.com/davehorner/cargo-e/tree/develop/addendum/e_obs](https://github.com/davehorner/cargo-e/tree/develop/addendum/e_obs) repository.

--dave horner
6/2025

## Warning

**Note**: Using `e_obs` will modify your desktop wallpaper background. A random screenshot from the last third of the previous command's screenshots will be set as the wallpaper. Ensure this behavior aligns with your preferences before using the tool.