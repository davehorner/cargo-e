
### Cool Examples That Deserve Mention

**cargo‑e** is built to work seamlessly with a wide variety of Rust projects—from simple programs to complex examples and binaries. Whether you're testing a creative project or a robust application, cargo‑e aims to make running and exploring your code as effortless as possible. Below are some cool examples that have been tried and tested on my machines.

Lets start with a great set of examples [altunenes/rusty_art](https://github.com/altunenes/rusty_art).  We will pay attention to how many Rustaceans might approach the exploration of these code bases.
- [altunenes/rusty_art](https://github.com/altunenes/rusty_art) - 3/25 86 binaries.
  This project has many binaries.  It follows `cargo --bin` conventions for its examples.  cargo run provides this long list. 
    ```bash
        rusty_art on  master [!?] is 📦 v0.1.0 via 🦀 v1.86.0-nightly
        ❯ cargo run
        error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
        available binaries: 2025tree, 3dneuron, GPUattractor, adelson, asahi, asahi2, attractors, bhole, blobs, butter2d, cafe_wall, chladni, chladniwgpu, darkclouds, dfft, dottedlines, expmandelbrotgpu, eyes, faketunnel, fbmflowgpu, fibonacci, fluid, footsteps, fourier, gaborill, gaborimage, gaborwgpu, galaxy, galaxy2, gaussiansplat, golf, hilbert, hilbertimg, hole, imgblob, imlenswgpu, kleinian, leviant, lilac, lorenz, lovewgpu, mandelbrot, mandelbulb, munker, munkerclock, nblur, nebula, neuralnet, neuralnetwgpu, neurons, neurons2, orbittraps, ornaments, oscillation, pdiamond, peace, peaceGPU, pina, pixelate, pixelflow, pixelrain, pixelsum, psychology, pupils, rainbowimage, rorschach, scramble, sdrect, sdvert, sinh, smoothneurons, smoothvoro, snowflake, snowflakewgpu, spiralimgwgpu, stripes, tree, triangles, tunnel, ulam, voronoi, voronoiwgpu, waves, waves2, winterflake, wrapper
    ```
That list is what you get for experience using barebones `cargo run`.  Find the name that looks good to you and type the whole name, because `cargo run` does not do partial matches.  Thank someone for copy/paste and tab completions, but those names are not exposed to your shell until after you've invoked it a few times.  I will often cd into the examples or bin folder to get tab completions.
    
If you run:
```bash
    ❯ cargo run --example
    error: "--example" takes one argument.
    Available examples:
        rusty_art_tui
```
You will see 1 example, that is a `rusty_art_tui` example precursor to `cargo-e`.  
    
    
Not everyone is aware of the examples folder, how to use it, and using `cargo run --example`.  In some ways, having all of the examples show up as binaries makes them more discoverable.  I do not know the reasons why one developer chooses to use standard bin vs standard example convention.  

**cargo-e** is about `OPC`.  structuring examples and binaries in built-in single file .rs files is a great cargo convention.

People do as they like, and they need more than a single file bin or ex., we often place the files in other locations, or use a directory as an example/binary package inside the examples.  _some ppl would disagre_, but there's no wrong way to do it.

**that's how other people like to structure and use their code.**




    

- [altunenes/cuneus](https://github.com/altunenes/cuneus) - 3/25 32 binaries.
  ```bash
    cargo run
        error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
        available binaries: asahi, attractor, clifford, dna, droste, fluid, galaxy, genuary2025_18, genuary2025_6, hilbert, lich, mandelbrot, mandelbulb, matrix, nebula, orbits, poe2, rorschach, roto, satan, sdvert, simplefeedback, sinh, spiral, tree, voronoi, xmas
  ```

- [nannou](https://github.com/nannou-org/nannou) - 3/25 - 0 built-in examples (214 alternatives: 208 examples, 6 binaries) in normal mode.
    ```bash
        nannou on  master [!?] via 🦀 v1.86.0-nightly
        ❯ cargo run
        error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
        available binaries: nannou_egui_demo_app, nannou_new, nannou_package, run_all_examples, set_version
    ```
- [iced](https://github.com/iced-rs/iced) - 3/25 0 built-in examples (52 alternatives: 0 examples, 52 binaries) examples folder has examples/[example]/src/main.rs 

Let's deep drive on a simple set of examples that will record well for an animated gif:
- [nu-ansi-term](https://github.com/nushell/nu-ansi-term) - 0 built-in examples (6 alternatives: 6 examples, 0 binaries).
```
nu-ansi-term/examples on  main via 🦀 v1.86.0-nightly
❯ tree
.
├── 256_colors.rs
├── basic_colors.rs
├── gradient_colors.rs
├── hyperlink.rs
├── may_sleep
│   └── mod.rs
├── rgb_colors.rs
└── title.rs
```
As mentioned in [auto-resolve-workspace-errors.md](auto-resolve-workspace-errors.md); its a problem if you want to use cargo on a "loose" project in your existing workspace.  cargo-e takes care of temporarily patching the project Cargo.toml so we can go back to using our code and folders as we would like within the existing workspace.
```bash
nu-ansi-term on  main is 📦 v0.50.1 via 🦀 v1.86.0-nightly
❯ cargo run --example
error: current package believes it's in a workspace when it's not:
current:   /Users/dhorner/w/cargo-e/documents/example_walkthrus/nu-ansi-term/Cargo.toml
workspace: /Users/dhorner/w/cargo-e/Cargo.toml

this may be fixable by adding `documents/example_walkthrus/nu-ansi-term` to the `workspace.members` array of the manifest located at: /Users/dhorner/w/cargo-e/Cargo.toml
Alternatively, to keep it out of the workspace, add the package to the `workspace.exclude` array, or add an empty `[workspace]` table to the package's manifest.
```
I don't know why the workspace identity crisis is cause for breaking the querying and running --bin and --example interface.  Someone decided that was the appropriate response to a "Loose" crate.  That's where `cargo` is today 3/25.

```bash
nu-ansi-term on  main [!?] is 📦 v0.50.1 via 🦀 v1.86.0-nightly took 18s
❯ cargo run
error: a bin target must be available for `cargo run`
```
This one uses the single file rust file --example convention, with the exception of one `may_sleep/mod.rs` which does not show up in the list of examples. 

`may_sleep/mod.rs` contains two fn `parse_cmd_args` and `sleep`.  `may_sleep/mod.rs` contains no `fn main` and is really not an example at all but a libraray for the examples.

This is not something you see everyday, and highlights the complexity of trying to define "extended" binaries and samples as whole directories within the `bin` or `examples` folder as many developers like.  You can define support libraries in the examples folder, you learn something new everyday.
```bash
nu-ansi-term on  main [!?] is 📦 v0.50.1 via 🦀 v1.86.0-nightly
❯ cargo run --example
error: "--example" takes one argument.
Available examples:
    256_colors
    basic_colors
    gradient_colors
    hyperlink
    rgb_colors
    title
```

Now, lets see what cargo-e does:
```bash
nu-ansi-term/examples on  main [!?] via 🦀 v1.86.0-nightly
❯ cargo e
'cargo-e' 0.1.17 is latest.
workspace: nu-ansi-term/nu-ansi-term/Cargo.toml []
package: nu-ansi-term/Cargo.toml
0 built-in examples (6 alternatives: 6 examples, 0 binaries).
== press q, t, wait for 3 seconds, or other key to continue.
  1: [ex.] 256_colors
  2: [ex.] basic_colors (1 run)
  3: [ex.] gradient_colors (1 run)
  4: [ex.] hyperlink (1 run)
  5: [ex.] rgb_colors (1 run)
  6: [ex.] title
* == # to run, tui, e<#> edit, 'q' to quit (waiting 5 seconds)
```

**cargo-e is a better experience.** This page isn't about going into the features, just a quick walk thru on a few examples to see how most people quickly use `cargo run` to find `--example` and `--bin`. 

This page will be updated with additional details as I have time to write and improve.

Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. 

>ChatGPT> Do a rough translation.
>Here's a rough translation:
>>"But I must explain to you how all this mistaken idea of denouncing pleasure and praising pain was born, and I will give you a complete account of the system, and expound the actual teachings of the great explorer of truth, the master-builder of human happiness."

Keep in mind that this version of the translation is more of an interpretative rendering than a literal word-for-word translation, as the original text is scrambled pseudo-Latin often used for placeholder purposes.


Your project, program, example, sample, or what have you should work fine with **cargo‑e**. If you know of other great examples, tests, or binaries that integrate well with cargo‑e, please share them. If **cargo‑e** doesn't work as expected with your project, I'd love to hear your feedback so we can make it even better.
