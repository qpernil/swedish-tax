# Swedish tax calculations for 2026

This crate implements Skatteverket monthly tax tables 32, 33, and 34 and the
annual preliminary-tax formulas from SKV 433, edition 36, for income year 2026.

The calculations use the same assumptions as the published tables and are not
an individualized final tax assessment.

## Command-line programs

```sh
cargo run --bin tax-annual -- 34 1 216000
cargo run --bin tax-monthly -- 34 1 18000
```

Arguments are tax table, column, and gross income in whole SEK.

## Sources

- [SKV 433 technical specification](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55c/1766385913260/teknisk-beskrivning-skv-433-2026-utgava-36.pdf)
- [Official monthly tables](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba5af/1765287119989/allmanna-tabeller-manad.txt)
- [Worked examples](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55f/1765284831853/bilaga-3-exempel-till-skv-433-2026.pdf)
