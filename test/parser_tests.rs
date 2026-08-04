use super::*;
use std::fs;
use std::path::{Path, PathBuf};

const EN: Lang = Lang::En;

fn test_data_path(name: &str) -> PathBuf {
    let candidates = [
        format!("test/test_data/{}", name),
        format!("../test/test_data/{}", name),
        format!("src/../test/test_data/{}", name),
    ];
    for c in &candidates {
        if Path::new(c).exists() {
            return PathBuf::from(c);
        }
    }
    PathBuf::from(&candidates[0])
}

/// Load the canonical valid product fixture. Every invalid-case test below
/// starts from this content and overrides a single piece of it to inject
/// the specific error scenario it exercises.
fn valid_content() -> String {
    let path = test_data_path("valid_product.txt");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e))
}

/// Parse `content` expecting failure and return the joined error messages.
fn parse_errs(content: &str) -> String {
    let errors = parse_content(content, &EN).expect_err("should fail");
    assert!(!errors.is_empty(), "expected at least one error");
    errors.join("\n")
}

#[test]
fn parses_a_valid_product_with_multiple_costs() {
    let content = "+ Cerveza : 2.7 USD : 0.2 mins\n\
                   - 0.27 USD labor humana\n\
                   - 0.32 USD cerveza\n\
                   - 0.10 USD gasto agua limpieza\n";
    let product = parse_content(content, &EN).expect("should parse");
    assert_eq!(product.name, "Cerveza");
    assert!((product.sale_price - 2.7).abs() < 1e-9);
    assert_eq!(product.sale_currency, Currency::new("USD"));
    assert!((product.production_time - 0.2).abs() < 1e-9);
    assert_eq!(product.production_time_unit, TimeUnit::Mins);
    assert_eq!(product.costs.len(), 3);
    assert!((product.costs[0].price - 0.27).abs() < 1e-9);
    assert_eq!(product.costs[0].currency, Currency::new("USD"));
    assert_eq!(product.costs[0].description, "labor humana");
    assert_eq!(product.costs[2].description, "gasto agua limpieza");
}

#[test]
fn parses_product_with_usd_and_hours() {
    let content = "+ Widget : 10 USD : 1.5 hours\n  - 3 USD parts\n";
    let product = parse_content(content, &EN).expect("should parse");
    assert_eq!(product.sale_currency, Currency::new("USD"));
    assert_eq!(product.production_time_unit, TimeUnit::Hours);
    assert_eq!(product.costs.len(), 1);
    assert_eq!(product.costs[0].currency, Currency::new("USD"));
}

#[test]
fn parses_product_with_cad_and_no_costs() {
    let content = "+ Maple : 5 CAD : 30 mins\n";
    let product = parse_content(content, &EN).expect("should parse");
    assert_eq!(product.sale_currency, Currency::new("CAD"));
    assert!(product.costs.is_empty());
}

#[test]
fn ignores_blank_lines() {
    let content = "\n+ Beer : 2 USD : 1 mins\n\n  - 1 USD hops\n\n";
    let product = parse_content(content, &EN).expect("should parse");
    assert_eq!(product.name, "Beer");
    assert_eq!(product.costs.len(), 1);
}

#[test]
fn rejects_invalid_sale_currency() {
    // "1XY" is not a valid ISO 4217 code (lowercase / non-alpha).
    let content = valid_content().replace("2.7 USD : 0.2 mins", "2.7 1XY : 0.2 mins");
    let errors = parse_errs(&content);
    assert!(errors.contains("invalid sale currency") && errors.contains("1XY"), "{}", errors);
}

#[test]
fn rejects_invalid_cost_currency() {
    let content = valid_content().replace("0.27 USD labor", "0.27 1XY labor");
    let errors = parse_errs(&content);
    assert!(errors.contains("invalid cost currency") && errors.contains("1XY"), "{}", errors);
}

#[test]
fn accepts_any_iso_4217_currency_code() {
    // GBP, JPY, MXN are all valid ISO 4217 codes that were previously rejected.
    let content = "+ Widget : 10 GBP : 1 mins\n  - 3 JPY parts\n  - 2 MXN labor\n";
    let product = parse_content(content, &EN).expect("should parse");
    assert_eq!(product.sale_currency.to_string(), "GBP");
    assert_eq!(product.costs[0].currency.to_string(), "JPY");
    assert_eq!(product.costs[1].currency.to_string(), "MXN");
}

#[test]
fn rejects_invalid_time_unit() {
    let content = valid_content().replace("0.2 mins", "0.2 seconds");
    let errors = parse_errs(&content);
    assert!(errors.contains("invalid production time unit") && errors.contains("seconds"), "{}", errors);
}

#[test]
fn rejects_non_numeric_sale_price() {
    let content = valid_content().replace("2.7 USD", "cheap USD");
    let errors = parse_errs(&content);
    assert!(errors.contains("invalid sale price"), "{}", errors);
}

#[test]
fn rejects_non_numeric_cost_price() {
    let content = valid_content().replace("0.27 USD labor", "cheap USD labor");
    let errors = parse_errs(&content);
    assert!(errors.contains("invalid cost price"), "{}", errors);
}

#[test]
fn rejects_non_numeric_production_time() {
    let content = valid_content().replace("0.2 mins", "fast mins");
    let errors = parse_errs(&content);
    assert!(errors.contains("invalid production time"), "{}", errors);
}

#[test]
fn rejects_line_with_bad_prefix() {
    let content = valid_content().replace("  - 0.27 USD labor", "  * 0.27 USD labor");
    let errors = parse_errs(&content);
    assert!(errors.contains("unexpected line"), "{}", errors);
}

#[test]
fn rejects_product_line_without_plus_prefix() {
    let content = valid_content().replace("+ Beer : 2.7 USD : 0.2 mins", "Beer : 2.7 USD : 0.2 mins");
    let errors = parse_errs(&content);
    assert!(errors.contains("unexpected line"), "{}", errors);
    assert!(errors.contains("missing product definition"), "{}", errors);
}

#[test]
fn rejects_cost_line_without_minus_prefix() {
    let content = valid_content().replace("  - 0.27 USD labor", "  0.27 USD labor");
    let errors = parse_errs(&content);
    assert!(errors.contains("unexpected line"), "{}", errors);
}

#[test]
fn rejects_missing_product_line() {
    let content = valid_content().replace("+ Beer : 2.7 USD : 0.2 mins\n", "");
    let errors = parse_errs(&content);
    assert!(errors.contains("missing product definition"), "{}", errors);
}

#[test]
fn rejects_empty_product_name() {
    let content = valid_content().replace("+ Beer : 2.7 USD : 0.2 mins", "+ : 2.7 USD : 0.2 mins");
    let errors = parse_errs(&content);
    assert!(errors.contains("product name is empty"), "{}", errors);
}

#[test]
fn rejects_cost_line_with_too_few_tokens() {
    let content = valid_content().replace("  - 0.27 USD labor", "  - 0.27 USD");
    let errors = parse_errs(&content);
    assert!(errors.contains("cost line must be"), "{}", errors);
}

#[test]
fn rejects_product_line_with_wrong_section_count() {
    let content = valid_content().replace(": 0.2 mins", " 0.2 mins");
    let errors = parse_errs(&content);
    assert!(errors.contains("colon-separated sections"), "{}", errors);
}

#[test]
fn rejects_sale_section_with_wrong_token_count() {
    let content = valid_content().replace("2.7 USD", "2.7USD");
    let errors = parse_errs(&content);
    assert!(errors.contains("sale section"), "{}", errors);
}

#[test]
fn rejects_time_section_with_wrong_token_count() {
    let content = valid_content().replace("0.2 mins", "0.2mins");
    let errors = parse_errs(&content);
    assert!(errors.contains("production-time section"), "{}", errors);
}

#[test]
fn rejects_duplicate_product_line() {
    let content = format!("{}\n+ Ale : 3 USD : 1 mins", valid_content());
    let errors = parse_errs(&content);
    assert!(errors.contains("duplicate product definition"), "{}", errors);
}

#[test]
fn parses_valid_test_data_file() {
    let path = test_data_path("valid_product.txt");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
    let product = parse_content(&content, &EN).expect("valid_product.txt should parse");
    assert_eq!(product.name, "Beer");
    assert_eq!(product.sale_currency, Currency::new("USD"));
    assert!(product.costs.len() >= 2);
}

#[test]
fn display_uses_selected_language() {
    let content = "+ Beer : 2 USD : 1 mins\n  - 1 USD hops\n";
    let product = parse_content(content, &EN).expect("should parse");
    let en = product.display(&Lang::En);
    assert!(en.contains("Product: Beer"), "en was: {}", en);
    assert!(en.contains("Sale price:"), "en was: {}", en);
    assert!(en.contains("Costs (1):"), "en was: {}", en);

    let es = product.display(&Lang::Es);
    assert!(es.contains("Producto: Beer"), "es was: {}", es);
    assert!(es.contains("Precio de venta:"), "es was: {}", es);

    // The localized time-unit label follows the chosen language.
    assert_eq!(product.production_time_unit.label(&Lang::En), "mins");
    assert_eq!(product.production_time_unit.label(&Lang::Es), "minutos");
    assert_eq!(product.production_time_unit.label(&Lang::Zh), "分钟");
}
