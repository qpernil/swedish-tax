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

The native egui application includes a reopenable income calculator for
salary, one-time compensation, occupational pension, and dividends within the
owner-managed company's gränsbelopp. It estimates the withholding applied by main
and secondary payers (including percentage adjustment decisions), reconciles
withholding with annual final tax, and estimates progress toward the 2026 PGI
and SGI ceilings. Regular salary rows can estimate an ITP 1-equivalent employer
pension premium, while one-time salary rows can model an editable salary exchange,
the employer uplift, the remaining pension allowance, and the taxable cash payment.

```sh
cargo run -p swedish-tax-gui
```

The GUI is a separate workspace package. Building the library or command-line
programs does not build the graphical application unless it is selected.

## Sources

- [SKV 433 technical specification](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55c/1766385913260/teknisk-beskrivning-skv-433-2026-utgava-36.pdf)
- [Official monthly tables](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba5af/1765287119989/allmanna-tabeller-manad.txt)
- [2026 withholding on one-time payments](https://www.skatteverket.se/foretag/arbetsgivare/arbetsgivaravgifterochskatteavdrag/skatteavdrag/engangsbelopp.4.361dc8c15312eff6fd3225e.html)
- [Worked examples](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55f/1765284831853/bilaga-3-exempel-till-skv-433-2026.pdf)
- [2026 pensionable income (PGI)](https://www.skatteverket.se/privat/skatter/arbeteochinkomst/pensionsgrundandeinkomstpgi.4.4f3d00a710cc9ae1c9c80008300.html)
- [Sickness-benefit qualifying income (SGI)](https://www.forsakringskassan.se/privatperson/sjukpenninggrundande-inkomst-sgi)
- [2026 ITP 1-equivalent pension-premium benchmark](https://collectum.se/avtal-och-faktura/faktura-och-premier/aktuella-premier-och-basbelopp)
- [Employer pension-cost deduction rules](https://www4.skatteverket.se/rattsligvagledning/edition/2026.3/339021.html)
