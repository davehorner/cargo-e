// extended/e_crate_version_checker/build.rs

use std::io::Write;

fn main() {
    // For example, tell Cargo to rerun this build script if Cargo.toml changes,
    // or if any file in the "src" directory changes.
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/");
    // If "changelog" feature is enabled, set default changelog path if not overridden
    if std::env::var("CARGO_FEATURE_CHANGELOG").is_ok()
        && std::env::var("E_CRATE_CHANGELOG_PATH").is_err()
    {
        println!("cargo:rustc-env=E_CRATE_CHANGELOG_PATH=../cargo-e.CHANGELOG.md");
    }

    // If "fortune" feature is enabled, select external file or generate default fortunes
    if std::env::var("CARGO_FEATURE_FORTUNE").is_ok() {
        // Try external override
        let ext = std::env::var("E_CRATE_FORTUNE_PATH").ok();
        let valid_ext = ext.as_ref().and_then(|p| {
            let path = std::path::Path::new(p);
            if path.is_file() && path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                Some(p.clone())
            } else {
                None
            }
        });
        // Determine final path: external if valid, else write defaults to OUT_DIR
        let final_path = if let Some(path) = valid_ext {
            path
        } else {
            let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
            let out_path = std::path::Path::new(&out_dir).join("fortunes.txt");
            let mut file = std::fs::File::create(&out_path)
                .expect("Failed to create default fortunes file in OUT_DIR");
            for line in default_fortunes() {
                writeln!(file, "{}", line).expect("Failed to write default fortune");
            }
            out_path.to_string_lossy().into_owned()
        };
        println!("cargo:rustc-env=E_CRATE_FORTUNE_PATH={}", final_path);
    }
}

/// Default fortunes used when genai integration is unavailable or for fallback
fn default_fortunes() -> Vec<String> {
    vec![
        // Original 10 fortunes, preserved in order
        String::from("Why do programmers prefer dark mode? Because light attracts bugs."),
        String::from("A SQL query walks into a bar, walks up to two tables and asks, 'Can I join you?'."),
        String::from("Why do Java developers wear glasses? Because they don't see sharp."),
        String::from("There are only 10 types of people in the world: those who understand binary and those who don't."),
        String::from("Debugging: Being the detective in a crime movie where you are also the murderer."),
        String::from("I would tell you a UDP joke, but you might not get it."),
        String::from("https://github.com/davehorner/cargo-e consider a star to show your support"),
        String::from("In a world without fences and walls, who needs Gates and Windows?"),
        String::from("'Knock, knock.' 'Who's there?' very long pause… 'Java.'"),
        String::from("Please consider giving this project a star on https://github.com/davehorner/cargo-e"),
        // 100 new programmer-themed, funny fortunes
        //x.com Grok3 5/25
        String::from("Why did the programmer quit? No *statistically significant* reason to stay."),
        String::from("My code has no bugs, just *undocumented features* in beta."),
        String::from("Tabs vs. spaces? I choose chaos: random indentation."),
        String::from("I said my code was 90% done. Now I’m trapped in the last 10% forever."),
        String::from("What’s a programmer’s favorite dance? The merge conflict shuffle."),
        String::from("Python devs hate JavaScript: too many braces, not enough zen."),
        String::from("I named my variable 'i' and now it’s having an identity crisis."),
        String::from("Why delete a Git branch? To *commit* to a fresh start."),
        String::from("My code works fine… until someone observes it. Quantum bugs!"),
        String::from("Spellcheck? Nah, `teh` is just *idiomatic* code."),
        String::from("Why do programmers prefer Vim? Because `hjkl` is their love language."),
        String::from("I tried to write a recursive joke, but I called it too many times."),
        String::from("Why did the CSS fail? It couldn’t find any *specificity*."),
        String::from("Rust programmers don’t panic; they just *unwrap* their feelings."),
        String::from("I told my boss I’d fix the bug… in the next sprint. Maybe."),
        String::from("Why do devs love coffee? It’s the only thing keeping the stack from overflowing."),
        String::from("My regex works perfectly… on the empty string."),
        String::from("Why did the database crash? It had an *identity* crisis during a join."),
        String::from("I don’t write bugs; I write *easter eggs* for QA."),
        String::from("What’s a coder’s favorite game? Git blame roulette."),
        String::from("Why did the function fail? It had too many *side effects* from partying."),
        String::from("I pushed to prod on Friday. Now I’m *404: Weekend Not Found*."),
        String::from("Why do programmers prefer dark mode? It hides their tears better."),
        String::from("My unit tests passed… after I commented out the failing ones."),
        String::from("Why did the coder go broke? They kept chasing *null* pointers."),
        String::from("I don’t use Stack Overflow. I just *grep* my soul for answers."),
        String::from("Why do C programmers live dangerously? They play with raw pointers."),
        String::from("My code’s so clean, it sparkles… until you run it."),
        String::from("Why did the dev refuse to pair program? They didn’t want to *share* the blame."),
        String::from("I wrote a one-liner in Perl. Now it’s running for president."),
        String::from("Why do front-end devs hate IE? It’s the browser that says, 'I’m special.'"),
        String::from("My API has 99% uptime… during scheduled maintenance."),
        String::from("Why did the programmer burn out? Too many *async* life tasks."),
        String::from("I don’t fear bugs. I fear the logs that hide them."),
        String::from("Why did the loop run forever? It was too *attached* to its condition."),
        String::from("My code’s modular… until you try to reuse it."),
        String::from("Why do Go programmers love simplicity? Because `if err != nil` is their mantra."),
        String::from("I tried to optimize my code, but now it’s just *prematurely optimized* sadness."),
        String::from("Why did the dev quit GitHub? Too many *forking* decisions."),
        String::from("My CI pipeline is green… because I disabled the tests."),
        String::from("Why do programmers prefer Linux? It’s the only OS that *listens* to their screams."),
        String::from("I wrote a bug-free program… in my dreams, using pseudocode."),
        String::from("Why did the array index start at 0? It wanted to keep a low profile."),
        String::from("My microservices are so small, they’re just *nano-regrets*."),
        String::from("Why do devs hate meetings? They’re just *synchronous* interruptions."),
        String::from("I don’t write documentation. My code is *self-explanatory*… to me, yesterday."),
        String::from("Why did the compiler complain? It was having a *syntax* tantrum."),
        String::from("My pull request was rejected. Guess I’ll just *stash* my ego."),
        String::from("Why do programmers prefer headphones? To mute the world’s runtime errors."),
        String::from("I tried to learn Haskell, but my brain kept asking for *side effects*."),
        String::from("Why did the dev go to therapy? Too many *unresolved dependencies*."),
        String::from("My code’s so legacy, it’s eligible for social security."),
        String::from("Why do Ruby devs love Rails? It’s the only framework that *feels* like a hug."),
        String::from("I don’t debug; I just *print* my way to victory."),
        String::from("Why did the programmer get lost? They followed a *relative* path."),
        String::from("My Docker container is so light, it’s just *hot air* and hope."),
        String::from("Why do devs fear prod? It’s where dreams go to *segfault*."),
        String::from("I wrote a thread-safe program… by avoiding threads entirely."),
        String::from("Why did the JSON fail? It was *malformed* and emotionally unstable."),
        String::from("My Agile team has daily standups… to practice for the sprint collapse."),
        String::from("Why do programmers prefer night shifts? The bugs are sleepier then."),
        String::from("I don’t use frameworks. I just *reinvent* the wheel, badly."),
        String::from("Why did the dev hate XML? Too many *pointy* brackets."),
        String::from("My code’s so DRY, it’s practically a desert."),
        String::from("Why did the database go offline? It needed a *schema* cleanse."),
        String::from("I tried pair programming, but my partner kept *refactoring* my personality."),
        String::from("Why do PHP devs love chaos? Because `<?php` is their battle cry."),
        String::from("My cloud bill is so high, I’m coding in the *stratosphere*."),
        String::from("Why did the programmer get fired? They kept *committing* to nothing."),
        String::from("I don’t write tests. I just *pray* to the CI gods."),
        String::from("Why do TypeScript devs feel safe? They’ve got *types* to protect them."),
        String::from("My app’s so slow, it’s practically a *feature freeze*."),
        String::from("Why did the dev hate regex? It’s like solving a puzzle with no edges."),
        String::from("I named my project ‘Done.’ Now it’s *ironic* every day."),
        String::from("Why do programmers love Rust? It’s the only language that *cares* about their safety."),
        String::from("My code’s so old, it’s written in *COBOL* and dreams of punch cards."),
        String::from("Why did the API return 500? It was having an *existential* crisis."),
        String::from("I don’t use version control. I just *email* myself ZIP files."),
        String::from("Why do devs love Kubernetes? It’s like herding *digital* cats."),
        String::from("My program crashed… but it looked *aesthetic* doing it."),
        String::from("Why did the coder switch to dark mode? To match their *soul*."),
        String::from("I tried to write a joke in Assembly, but it was just *MOV*ing sadness."),
        String::from("Why do programmers hate deadlines? They’re just *arbitrary* segfaults."),
        String::from("My code’s so fragile, it breaks when you *look* at it."),
        String::from("Why did the dev use NoSQL? They wanted to live *schemaless* and free."),
        String::from("I don’t fix bugs; I just *work around* them creatively."),
        String::from("Why do programmers love memes? They’re the only thing *compiled* faster than code."),
        String::from("My IDE crashed. Now I’m coding in *Notepad* like it’s 1999."),
        String::from("Why did the function return null? It was *socially distancing*."),
        String::from("I tried to learn functional programming, but I kept *mutating* my mindset."),
        String::from("Why do programmers prefer silence? Because *noise* is just unhandled exceptions."),
        String::from("My backlog’s so long, it’s practically a *linked list* to nowhere."),
        String::from("Why did the dev hate Java? Too many *boilerplate* emotions."),
        String::from("I don’t write comments. My code is a *mystery novel* for future me."),
        String::from("Why do programmers love open source? It’s like free bugs for everyone!"),
        String::from("My app’s so secure, even *I* can’t log in."),
        String::from("Why did the coder go to art school? To learn how to *draw* a better stack trace."),
        String::from("I don’t fear memory leaks. I fear the *void* they leave behind."),
        String::from("Why do programmers prefer dark mode? It’s easier to hide their *syntax errors*."),
    ]
}