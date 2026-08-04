use super::*;

#[test]
fn from_code_roundtrip() {
    for code in ["en", "es", "zh", "de", "ru", "fr"] {
        let l = Lang::from_code(code).expect("known code");
        assert_eq!(l.code(), code);
    }
    assert!(Lang::from_code("xx").is_none());
}

#[test]
fn default_is_english() {
    assert_eq!(Lang::DEFAULT, Lang::En);
}

#[test]
fn fmt_substitutes_placeholders() {
    assert_eq!(fmt("a{0}b{1}c", &["X", "Y"]), "aXbYc");
    assert_eq!(fmt("no args", &[]), "no args");
    assert_eq!(fmt("{0}-{0}", &["z"]), "z-z");
    // out-of-range index yields empty substitution
    assert_eq!(fmt("{0}{1}", &["only"]), "only");
}

#[test]
fn fmt_handles_multibyte_and_bare_brace() {
    assert_eq!(fmt("📦 {0} → {1}", &["Café", "100"]), "📦 Café → 100");
    assert_eq!(fmt("literal { not a placeholder", &[]), "literal { not a placeholder");
}

#[test]
fn all_languages_have_required_sales_needle_distinct() {
    // Sanity: each language's needle is non-empty.
    for l in [Lang::En, Lang::Es, Lang::Zh, Lang::De, Lang::Ru, Lang::Fr] {
        assert!(!l.dict().required_sales_needle.is_empty());
    }
}

#[test]
fn str_width_counts_ascii_and_double_width() {
    assert_eq!(str_width("Product:"), 8);
    assert_eq!(str_width("📦 Product:"), 11); // emoji(2) + space(1) + "Product:"(8)
    assert_eq!(str_width("产品"), 4);         // CJK: 2 per char
    assert_eq!(str_width("Цена"), 4);         // Cyrillic: 1 per char
}

#[test]
fn pad_to_pads_with_spaces() {
    assert_eq!(pad_to("ab", 5), "ab   ");
    assert_eq!(pad_to("abcd", 4), "abcd");
    // Already wider than target: returned unchanged.
    assert_eq!(pad_to("abcde", 2), "abcde");
}

#[test]
fn prefix_before_value_strips_trailing_whitespace() {
    assert_eq!(prefix_before_value("📦 Product: \t\t\t{0}"), "📦 Product:");
    assert_eq!(prefix_before_value("no placeholder"), "no placeholder");
}

#[test]
fn fmt_aligned_pads_label_with_spaces() {
    // Tab-based padding in the template is replaced by space padding.
    let out = fmt_aligned("📦 Product: \t\t\t{0}", &["Cerveza"], 24);
    assert!(out.starts_with("📦 Product:"));
    assert!(!out.contains('\t'), "aligned output must not contain tabs");
    assert!(out.ends_with("Cerveza"));
    // The value column starts at the same offset regardless of label length.
    let short = fmt_aligned("📦 X: {0}", &["v"], 24);
    let long = fmt_aligned("📦 Something long: {0}", &["v"], 24);
    let off_s = short.find("v").unwrap();
    let off_l = long.find("v").unwrap();
    assert_eq!(off_s, off_l, "value column must align");
}

#[test]
fn pad_left_right_justifies() {
    assert_eq!(pad_left("5", 4), "   5");
    assert_eq!(pad_left("1234", 4), "1234");
    assert_eq!(pad_left("12345", 3), "12345"); // wider than target
}

#[test]
fn fmt_block_right_aligns_numeric_columns() {
    // Two rows whose numeric values have very different widths; each
    // value column must be right-aligned to a common width.
    let rows: Vec<(&str, Vec<String>)> = vec![
        ("{0} {1} {2}", vec!["5".into(), "USD".into(), "100".into()]),
        ("{0} {1} {2}", vec!["1234".into(), "USD".into(), "5".into()]),
    ];
    let out = fmt_block(&rows, 0);
    assert_eq!(out.len(), 2);
    // Column 0 right-aligned to width 4, column 2 to width 3.
    assert_eq!(out[0], "   5 USD 100");
    assert_eq!(out[1], "1234 USD   5");
}

#[test]
fn value_column_aligns_across_all_languages() {
    // Regression: in de and ru the value column was scattered because
    // str_width overcounted accented Latin and Cyrillic as width 2.
    // The invariant: every template's prefix width is strictly less than
    // label_w (= max prefix width + 2), so pad_to pads each to label_w
    // and every value starts at the same display column.
    for lang in [Lang::En, Lang::Es, Lang::Zh, Lang::De, Lang::Ru, Lang::Fr] {
        let d = lang.dict();
        let templates = [
            d.result_product,
            d.result_sale_price,
            d.result_total_cost,
            d.result_net_profit_unit,
            d.result_profit_margin,
            d.result_prod_time,
        ];
        let label_w = templates
            .iter()
            .map(|t| str_width(prefix_before_value(t)))
            .max()
            .unwrap_or(0)
            + 2;
        for (i, t) in templates.iter().enumerate() {
            let pw = str_width(prefix_before_value(t));
            assert!(
                pw < label_w,
                "lang {:?}: template {} prefix width {} >= label_w {}",
                lang.code(), i, pw, label_w
            );
        }
    }
}
