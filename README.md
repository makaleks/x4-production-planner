# X4 production planner / Производственный планировщик для X4

This planner takes desired productivity on input and provides total modules
names and counts on output. The planner uses game directory to work with actual
ballance.

## WIP note

This took longer than I expected, so now I take break with the published MVP.

The MVP works in terminal, takes output language, priorities and placklists and
prints csv that can be pasted in MS Excel/LibreOffice Calc for pretty view.

The unimplemented wishes include config file with various long-term-oriented
patches support for translations and program logic, WASM UI with i18n and
web-server. And more planner features like station states&patches generation
and food&medicaments&houses&ships planning, printing lists of wares. Got stuck
with serde custom mapping details and `toml_edit` consistency.

Have a look how does it work for now:

https://github.com/makaleks/x4-production-planner/assets/6663351/7ddcae08-c015-4203-9586-35b979e56cba

## Building

Until I add CI I assume you are able to build Rust code
(keywords: `rustup` and `cargo run`), so here I put notes on static linking.

[Linux](https://doc.bccnsoft.com/docs/rust-1.36.0-docs-html/edition-guide/rust-2018/platform-and-target-support/musl-support-for-fully-static-binaries.html):
```sh
rustup target add x86_64-unknown-linux-musl
```

On
[Windows-MSVC](https://www.reddit.com/r/rust/comments/l5mwdu/comment/gkv7em3/?utm_source=share&utm_medium=web3x&utm_name=web3xcss&utm_term=1&utm_content=share_button)
all should be added automatically as I have already added
```
#.cargo/config
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

## Enjoy!
