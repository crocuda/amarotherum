# Amarotherum - Arrogant web browser.

Web browser based on the [servo](https://servo.org) web engine and the [iced](https://iced.rs) GUI framework.

## Early development stage

For now Amarotherum is nothing near from usable.
It only displays hardcoded url, has neither mouse nor keyboard support.

See for yourself.

![screenshot](public/screenshot.png?raw=true)

## Useful commands

```sh
cargo build

# or

cargo run --bin amarotherum gui run
```

## Roadmap

- [x]: display a web page.

- [x]: integrate iced with servo runtime
  - [x]: display with gpu only.

- [x]: autodetect OS dark/light mode.

- [ ]: add configuration/theming support (TOML).

- [ ]: add command to search.
- [ ]: add keyboard support.

- [ ]: add mouse support.
