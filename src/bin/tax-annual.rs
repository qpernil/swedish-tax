use std::{env, process::ExitCode};
use swedish_tax::{annual_tax, Column};

fn parse_column(value: &str) -> Result<Column, String> {
    match value {
        "1" => Ok(Column::Column1),
        "2" => Ok(Column::Column2),
        "3" => Ok(Column::Column3),
        "4" => Ok(Column::Column4),
        "5" => Ok(Column::Column5),
        "6" => Ok(Column::Column6),
        _ => Err(format!("invalid column: {value}")),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(format!("usage: {} TABLE COLUMN YEARLY_INCOME_SEK", args[0]));
    }
    let table = args[1]
        .parse::<u8>()
        .map_err(|_| format!("invalid table: {}", args[1]))?;
    let column = parse_column(&args[2])?;
    let income = args[3]
        .parse::<u32>()
        .map_err(|_| format!("invalid yearly income: {}", args[3]))?;
    let tax = annual_tax(table, income, column)
        .ok_or_else(|| format!("unsupported tax table: {table}"))?;

    println!("assessed_income: {} SEK", tax.assessed_income);
    println!("basic_allowance: {} SEK", tax.basic_allowance);
    println!("taxable_income: {} SEK", tax.taxable_income);
    println!("state_income_tax: {} SEK", tax.state_income_tax);
    println!("municipal_income_tax: {} SEK", tax.municipal_income_tax);
    println!(
        "burial_and_religious_fee: {} SEK",
        tax.burial_and_religious_fee
    );
    println!("pension_fee: {} SEK", tax.pension_fee);
    println!("pension_fee_credit: {} SEK", tax.pension_fee_credit);
    println!("work_income_credit: {} SEK", tax.work_income_credit);
    println!(
        "sickness_compensation_credit: {} SEK",
        tax.sickness_compensation_credit
    );
    println!("earned_income_credit: {} SEK", tax.earned_income_credit);
    println!("public_service_fee: {} SEK", tax.public_service_fee);
    println!("total: {} SEK", tax.total);
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
