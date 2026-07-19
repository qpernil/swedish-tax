# Swedish tax calculations for 2026

This crate implements Skatteverket monthly tax tables 29 through 42 and the
annual preliminary-tax formulas from SKV 433, edition 36, for income year 2026.

The calculations use the same assumptions as the published tables and are not
an individualized final tax assessment.

## Command-line programs

```sh
cargo run --bin tax-annual -- 34 1 216000
cargo run --bin tax-monthly -- 34 1 18000
```

Arguments are tax table, column, and gross income in whole SEK.

## Desktop application

The native egui application provides monthly and annual income modes, tax
table and column selection, the official monthly table result, an annual
formula comparison, and the complete annual tax breakdown.

```sh
cargo run -p swedish-tax-gui
```

The GUI is a separate workspace package. Building the library or command-line
programs does not build the graphical application unless it is selected.

## Sources

- [SKV 433 technical specification](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55c/1766385913260/teknisk-beskrivning-skv-433-2026-utgava-36.pdf)
- [Official monthly tables](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba5af/1765287119989/allmanna-tabeller-manad.txt)
- [Worked examples](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55f/1765284831853/bilaga-3-exempel-till-skv-433-2026.pdf)
