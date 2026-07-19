use std::{env, fmt::Write as _, fs, path::PathBuf};

const SOURCE: &str = "data/allmanna-tabeller-manad-2026.txt";
const FIRST_TABLE: u8 = 29;
const LAST_TABLE: u8 = 42;
const RECORD_COUNT: usize = 7_966;
const RECORD_LENGTH: usize = 49;

#[derive(Clone, Copy)]
enum Kind {
    Amount,
    Percent,
}

struct Row {
    minimum: u32,
    maximum: u32,
    values: [u32; 6],
    kind: Kind,
}

fn main() {
    println!("cargo:rerun-if-changed={SOURCE}");

    let source = fs::read_to_string(SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {SOURCE}: {error}"));
    let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
    let table_count = usize::from(LAST_TABLE - FIRST_TABLE + 1);
    let mut tables: Vec<Vec<Row>> = (0..table_count).map(|_| Vec::new()).collect();

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        assert_eq!(
            line.len(),
            RECORD_LENGTH,
            "{SOURCE}:{line_number}: expected {RECORD_LENGTH} ASCII bytes"
        );
        assert_eq!(
            field(line, 0, 2, line_number),
            "30",
            "{SOURCE}:{line_number}: unsupported record type"
        );

        let kind = match field(line, 2, 3, line_number) {
            "B" => Kind::Amount,
            "%" => Kind::Percent,
            value => panic!("{SOURCE}:{line_number}: unsupported row kind {value:?}"),
        };
        let table = number(field(line, 3, 5, line_number), line_number, "table");
        let table = u8::try_from(table)
            .unwrap_or_else(|_| panic!("{SOURCE}:{line_number}: table does not fit in u8"));
        assert!(
            (FIRST_TABLE..=LAST_TABLE).contains(&table),
            "{SOURCE}:{line_number}: table {table} is outside {FIRST_TABLE}..={LAST_TABLE}"
        );

        let minimum = number(field(line, 5, 12, line_number), line_number, "minimum");
        let maximum_field = field(line, 12, 19, line_number).trim();
        let maximum = if maximum_field.is_empty() {
            u32::MAX
        } else {
            number(maximum_field, line_number, "maximum")
        };
        let mut values = [0; 6];
        for (index, value) in values.iter_mut().enumerate() {
            let start = 19 + index * 5;
            *value = number(
                field(line, start, start + 5, line_number),
                line_number,
                "column value",
            );
        }

        tables[usize::from(table - FIRST_TABLE)].push(Row {
            minimum,
            maximum,
            values,
            kind,
        });
    }

    assert_eq!(
        source.lines().count(),
        RECORD_COUNT,
        "{SOURCE}: unexpected record count"
    );
    for (offset, rows) in tables.iter().enumerate() {
        let table = usize::from(FIRST_TABLE) + offset;
        assert_eq!(
            rows.first().map(|row| row.minimum),
            Some(1),
            "{SOURCE}: table {table} does not start at income 1"
        );
        assert_eq!(
            rows.last().map(|row| row.maximum),
            Some(u32::MAX),
            "{SOURCE}: table {table} has no open-ended final row"
        );
        for pair in rows.windows(2) {
            assert_eq!(
                pair[0].maximum.checked_add(1),
                Some(pair[1].minimum),
                "{SOURCE}: table {table} contains a gap or overlap"
            );
        }
    }

    let output = render(&tables);
    let output_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("monthly_tables.rs");
    fs::write(&output_path, output)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", output_path.display()));
}

fn field(line: &str, start: usize, end: usize, line_number: usize) -> &str {
    line.get(start..end).unwrap_or_else(|| {
        panic!("{SOURCE}:{line_number}: field {start}..{end} is not valid ASCII")
    })
}

fn number(value: &str, line_number: usize, name: &str) -> u32 {
    value
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("{SOURCE}:{line_number}: invalid {name} {value:?}"))
}

fn render(tables: &[Vec<Row>]) -> String {
    let mut output = format!("const TABLES: [&[Row]; {}] = [\n", tables.len());
    for rows in tables {
        output.push_str("    &[\n");
        for row in rows {
            let constructor = match row.kind {
                Kind::Amount => "amount",
                Kind::Percent => "percent",
            };
            let maximum = if row.maximum == u32::MAX {
                "u32::MAX".to_owned()
            } else {
                row.maximum.to_string()
            };
            writeln!(
                output,
                "        {constructor}({}, {maximum}, [{}, {}, {}, {}, {}, {}]),",
                row.minimum,
                row.values[0],
                row.values[1],
                row.values[2],
                row.values[3],
                row.values[4],
                row.values[5],
            )
            .unwrap();
        }
        output.push_str("    ],\n");
    }
    output.push_str("];\n");
    output
}
