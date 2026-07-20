use std::{env, process::ExitCode};
use swedish_tax::{monthly_deduction, TaxColumn, TaxDeduction};

fn parse_column(value: &str) -> Result<TaxColumn, String> {
    match value {
        "1" => Ok(TaxColumn::Column1),
        "2" => Ok(TaxColumn::Column2),
        "3" => Ok(TaxColumn::Column3),
        "4" => Ok(TaxColumn::Column4),
        "5" => Ok(TaxColumn::Column5),
        "6" => Ok(TaxColumn::Column6),
        _ => Err(format!("invalid column: {value}")),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(format!(
            "usage: {} TABLE COLUMN MONTHLY_INCOME_SEK",
            args[0]
        ));
    }
    let table = args[1]
        .parse::<u8>()
        .map_err(|_| format!("invalid table: {}", args[1]))?;
    let column = parse_column(&args[2])?;
    let income = args[3]
        .parse::<u32>()
        .map_err(|_| format!("invalid monthly income: {}", args[3]))?;
    let deduction = monthly_deduction(table, column, income)
        .ok_or_else(|| format!("unsupported tax table: {table}"))?;

    match deduction {
        TaxDeduction::Amount(amount) => {
            println!("kind: amount");
            println!("amount: {amount} SEK");
        }
        TaxDeduction::Percent(percent) => {
            println!("kind: percent");
            println!("percent: {percent}.00%");
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
