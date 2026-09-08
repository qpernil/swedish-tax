//! Shared, transport-independent fixture generation for client parity tests.

use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path};

mod cases;
mod reference;

pub(super) fn run(workspace: &Path, check: bool) -> Result<(), String> {
    let root = workspace.join("tests/fixtures");
    let expected: BTreeMap<_, _> = cases::cases()
        .into_iter()
        .map(|(name, request)| {
            let response = reference::response(request.clone());
            let fixture = json!({"request": request, "response": response});
            (
                format!("{name}.json"),
                serde_json::to_string_pretty(&fixture).expect("JSON values serialize") + "\n",
            )
        })
        .collect();
    if check {
        let actual: BTreeMap<_, _> = fs::read_dir(&root)
            .map_err(|e| e.to_string())?
            .map(|entry| entry.map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .map(|entry| {
                fs::read_to_string(entry.path())
                    .map(|content| (entry.file_name().to_string_lossy().into_owned(), content))
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<_, _>>()?;
        if actual != expected {
            return Err(
                "Shared fixture drift; review changes and run `cargo xtask fixtures`".into(),
            );
        }
    } else {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        for (name, content) in &expected {
            fs::write(root.join(name), content).map_err(|e| e.to_string())?;
        }
    }
    println!(
        "{} {} shared fixtures",
        if check { "Checked" } else { "Generated" },
        expected.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reference::response;
    use serde_json::Value;

    #[test]
    fn checked_in_fixtures_match_native_reference() {
        run(
            Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
            true,
        )
        .unwrap();
    }

    #[test]
    fn invalid_fixture_shapes_and_duplicate_ids_are_typed_errors() {
        let mut invalid = cases::cases()[0].1.clone();
        invalid["plan"] = Value::Null;
        assert_eq!(response(invalid)["issue"]["kind"], "InvalidRequest");
        let mut v = cases::cases()[0].1.clone();
        let entry = v["plan"]["entries"][0].clone();
        v["plan"]["entries"].as_array_mut().unwrap().push(entry);
        assert_eq!(response(v.clone())["issue"]["kind"], "InvalidRequest");
        v["plan"]["entries"] = json!([]);
        assert_eq!(response(v.clone())["issue"]["kind"], "InvalidRequest");
    }
    #[test]
    fn reference_covers_calculation_and_editor_values() {
        let v = response(cases::cases()[1].1.clone());
        assert!(v["issue"].is_null());
        let result = &v["result"];
        assert_eq!(result["entries"][0]["vacation_compensation_amount"], 124992);
        assert_eq!(
            result["entries"][1]["allowance"]["pension_salary_basis_before"],
            1116000
        );
        assert_eq!(
            result["entries"][1]["allowance"]["pension_contributions_before"],
            210000
        );
        assert_eq!(result["calculation"]["salary_exchange_sacrifice"], 100000);
        assert_eq!(
            result["dividend"]["allowance"]["acquisition_cost_interest"],
            11550
        );
        assert!(result["calculation"]["adjustment_balance_trace"].is_object());
    }
}
