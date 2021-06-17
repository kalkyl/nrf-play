# `nRF52840-DK` playground

[`probe-run`] + [`defmt`] + [`flip-link`] + [`rtic`] Rust embedded playground

[`probe-run`]: https://crates.io/crates/probe-run
[`defmt`]: https://github.com/knurling-rs/defmt
[`flip-link`]: https://github.com/knurling-rs/flip-link
[`rtic`]: https://github.com/rtic-rs/cortex-m-rtic

## Dependencies

#### 1. `flip-link`:

```console
$ cargo install flip-link
```

#### 2. `probe-run`:

```console
$ cargo install probe-run
```

## Run!

Start by `cargo run`-ning `src/bin/blink.rs`:

```console
$ # `rb` is an alias for `run --bin`
$ cargo rb blink
  Finished dev [optimized + debuginfo] target(s) in 0.3s
  Running `probe-run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/blink`
  (HOST) INFO  flashing program (13.39 KiB)
  (HOST) INFO  success!
────────────────────────────────────────────────────────────────────────────────
0.000000 INFO  Hello world!
└─ blink::init @ src/bin/blink.rs:38
(..)
```
