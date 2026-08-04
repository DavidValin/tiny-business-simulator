use super::*;
use std::env;
use crate::parser::Cost;

const EN: Lang = Lang::En;

#[test]
fn result_file_path_appends_resultado() {
    let p = Path::new("test/test_data/foo.txt");
    let r = result_file_path(p);
    assert_eq!(r, Path::new("test/test_data/foo.simulation_results.txt"));
}

#[test]
fn compute_result_sums_costs_and_converts_hours() {
    let product = ProductDefinition {
        name: "X".into(),
        sale_price: 10.0,
        sale_currency: Currency::new("USD"),
        production_time: 1.5,
        production_time_unit: TimeUnit::Hours,
        costs: vec![
            Cost { price: 2.0, currency: Currency::new("USD"), description: "a".into() },
            Cost { price: 3.0, currency: Currency::new("USD"), description: "b".into() },
        ],
    };
    let r = compute_result(&product);
    assert!((r.total_cost - 5.0).abs() < 1e-9);
    assert!((r.net_profit - 5.0).abs() < 1e-9);
    assert!((r.profit_percent - 50.0).abs() < 1e-9);
    assert!((r.duration_minutes - 90.0).abs() < 1e-9);
    assert_eq!(r.currency, Currency::new("USD"));
}

fn make_result(name: &str, net_profit: f64, duration_minutes: f64, currency: Currency) -> ProductResult {
    ProductResult {
        name: name.into(),
        price: net_profit,
        currency,
        total_cost: 0.0,
        net_profit,
        profit_percent: 100.0,
        duration_minutes,
    }
}

#[test]
fn compute_product_shares_normalizes_to_100_percent() {
    let a = make_result("A", 6.0, 5.0, Currency::new("USD"));
    let b = make_result("B", 5.0, 10.0, Currency::new("USD"));
    let results: Vec<&ProductResult> = vec![&a, &b];

    // Raw rolls 70 / 30 -> normalized 0.7 / 0.3.
    let shares = compute_product_shares(&results, &[70, 30], 1000.0, 12000.0);

    assert_eq!(shares.len(), 2);
    assert_eq!(shares[0].raw_percent, 70);
    assert_eq!(shares[1].raw_percent, 30);

    let sum: f64 = shares.iter().map(|s| s.share).sum();
    assert!((sum - 1.0).abs() < 1e-9, "shares must sum to 1.0, got {}", sum);
    assert!((shares[0].share - 0.7).abs() < 1e-9);
    assert!((shares[1].share - 0.3).abs() < 1e-9);

    // Goal is split per the normalized share (currencies ignored).
    assert!((shares[0].monthly_goal - 700.0).abs() < 1e-9);
    assert!((shares[1].monthly_goal - 300.0).abs() < 1e-9);
    assert!((shares[0].annual_goal - 8400.0).abs() < 1e-9);
    assert!((shares[1].annual_goal - 3600.0).abs() < 1e-9);

    // Sales = ceil(share_goal / net_profit).
    assert_eq!(shares[0].monthly_sales, 117); // ceil(700/6)
    assert_eq!(shares[1].monthly_sales, 60);  // ceil(300/5)
    assert_eq!(shares[0].annual_sales, 1400); // ceil(8400/6)
    assert_eq!(shares[1].annual_sales, 720);  // ceil(3600/5)

    // Minutes = duration * sales.
    assert!((shares[0].monthly_minutes - 5.0 * 117.0).abs() < 1e-9);
    assert!((shares[1].monthly_minutes - 10.0 * 60.0).abs() < 1e-9);
}

#[test]
fn compute_product_shares_handles_extreme_rolls_and_zero_goal() {
    let a = make_result("A", 10.0, 1.0, Currency::new("USD"));
    let b = make_result("B", 10.0, 1.0, Currency::new("USD"));
    let results: Vec<&ProductResult> = vec![&a, &b];

    // Extremes of the allowed range: 1 and 70.
    let shares = compute_product_shares(&results, &[1, 70], 710.0, 0.0);

    assert!((shares[0].share - 1.0 / 71.0).abs() < 1e-9);
    assert!((shares[1].share - 70.0 / 71.0).abs() < 1e-9);
    // 710/71 = 10 -> 1 sale; 710*70/71 = 700 -> 70 sales.
    assert_eq!(shares[0].monthly_sales, 1);
    assert_eq!(shares[1].monthly_sales, 70);
    // Annual goal of 0 -> 0 sales.
    assert_eq!(shares[0].annual_sales, 0);
    assert_eq!(shares[1].annual_sales, 0);
}

#[test]
#[should_panic(expected = "same length")]
fn compute_product_shares_panics_on_length_mismatch() {
    let a = make_result("A", 10.0, 1.0, Currency::new("USD"));
    let results: Vec<&ProductResult> = vec![&a];
    let _ = compute_product_shares(&results, &[1, 2], 100.0, 0.0);
}

#[test]
fn parallel_range_zero_minutes_forces_one() {
    assert_eq!(parallel_range(0.0, 0.0, 8), (1, 1));
}

#[test]
fn parallel_range_monthly_binding_cap_to_one_workday() {
    // 60 h monthly, 365 h annual, 1 h/workday: monthly binds.
    // min = 60/30 = 2 (30 workdays), max = 60/1 = 60 (1 workday).
    assert_eq!(parallel_range(60.0 * 60.0, 365.0 * 60.0, 1), (2, 60));
}

#[test]
fn parallel_range_annual_binding() {
    // 10 h monthly, 200 h annual, 8 h/workday: annual binds.
    // min = ceil(200/(365*8)) = 1, max = floor(200/8) = 25.
    assert_eq!(parallel_range(10.0 * 60.0, 200.0 * 60.0, 8), (1, 25));
}

#[test]
fn parallel_range_typical_multi_product_totals() {
    // 800 h monthly, 1600 h annual, 8 h/workday: monthly binds.
    // min = ceil(800/240) = 4, max = floor(800/8) = 100.
    assert_eq!(parallel_range(800.0 * 60.0, 1600.0 * 60.0, 8), (4, 100));
}

#[test]
fn parallel_range_min_never_exceeds_max() {
    let (min, max) = parallel_range(100.0, 999_999.0, 3);
    assert!(min <= max && min >= 1, "got ({min}, {max})");
}

/// Byte offset of the value column for a `fmt_aligned` line: since the
/// label is space-padded to `label_w`, the value always starts at byte
/// offset `label_w` (unless the label itself was already wider, in which
/// case no padding was added and the value starts right after the label).
fn value_offset(line: &str, label_w: usize) -> usize {
    line.char_indices()
        .enumerate()
        .skip_while(|(i, (_, c))| *i < label_w.min(line.chars().count()) && *c == ' ')
        .next()
        .map(|(_, (bi, _))| bi)
        .unwrap_or(line.len())
}

#[test]
fn print_product_stats_aligns_value_column() {
    // Two products whose labels have very different widths; their value
    // columns must start at the same byte offset.
    let r1 = make_result("Cerveza", 2.01, 0.2, Currency::new("USD"));
    let r1 = ProductResult {
        price: 2.7,
        total_cost: 0.69,
        profit_percent: 74.44,
        ..r1
    };
    let r2 = make_result("American burger", 10.88, 12.0, Currency::new("USD"));
    let r2 = ProductResult {
        price: 16.0,
        total_cost: 5.12,
        profit_percent: 68.0,
        ..r2
    };

    let cases = [&r1, &r2];
    let mut prev_off: Option<usize> = None;
    for r in &cases {
        let d = Lang::En.dict();
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
            .map(|t| lang::str_width(lang::prefix_before_value(t)))
            .max()
            .unwrap_or(0)
            + 2;
        let rows: Vec<Vec<String>> = vec![
            vec![r.name.clone()],
            vec![format!("{:.2}", r.price), r.currency.to_string()],
            vec![format!("{:.2}", r.total_cost), r.currency.to_string()],
            vec![format!("{:.2}", r.net_profit), r.currency.to_string()],
            vec![format!("{:.2}", r.profit_percent)],
            vec![format!("{:.2}", r.duration_minutes)],
        ];
        let rendered: Vec<String> = templates
            .iter()
            .zip(rows.iter())
            .map(|(t, row)| {
                let refs: Vec<&str> = row.iter().map(String::as_str).collect();
                lang::fmt_aligned(t, &refs, label_w)
            })
            .collect();

        // No tabs anywhere — alignment is terminal-independent.
        for line in &rendered {
            assert!(!line.contains('\t'), "stats line must not contain tabs: {line:?}");
        }

        // Within one product, all six value columns must align.
        let first_off = value_offset(&rendered[0], label_w);
        for line in &rendered {
            assert_eq!(
                value_offset(line, label_w),
                first_off,
                "stats value columns must align:\n{}",
                rendered.join("\n")
            );
        }
        if let Some(po) = prev_off {
            assert_eq!(first_off, po, "value column must be identical across products");
        }
        prev_off = Some(first_off);
    }
}

#[test]
fn per_product_and_totals_lines_align_value_column() {
    let d = Lang::En.dict();
    // Pad product names to the longest name's width.
    let names = ["Cerveza", "American burger"];
    let name_w = names.iter().map(|n| lang::str_width(n)).max().unwrap();
    let padded: Vec<String> = names.iter().map(|n| lang::pad_to(n, name_w)).collect();

    // split_item: the 🎲 column starts at the same offset for both rows.
    let split_lines: Vec<String> = padded
        .iter()
        .map(|p| lang::fmt(d.split_item, &[p, "5", "9.26"]))
        .collect();
    for l in &split_lines {
        assert!(!l.contains('\t'), "split_item must not contain tabs: {l:?}");
    }
    let dice_offsets: Vec<usize> = split_lines
        .iter()
        .map(|l| l.find('🎲').expect("dice emoji present"))
        .collect();
    assert_eq!(
        dice_offsets[0], dice_offsets[1],
        "🎲 column must align across product names of different lengths"
    );

    // per_product_monthly / per_product_annual: every numeric column must
    // right-align across all rows in the block (this is the case the user
    // reported: `254.24` vs `5084.75` shifted the following columns).
    let per_w = [d.per_product_monthly, d.per_product_annual]
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;
    let per_rows: Vec<(&'static str, Vec<String>)> = vec![
        (d.per_product_monthly, vec!["254.24".into(), "USD".into(), "24".into(), "288.00".into(), "4.80".into()]),
        (d.per_product_annual,   vec!["5084.75".into(), "USD".into(), "468".into(), "5616.00".into(), "93.60".into()]),
        (d.per_product_monthly, vec!["4745.76".into(), "USD".into(), "493".into(), "493.00".into(), "8.22".into()]),
        (d.per_product_annual,   vec!["94915.25".into(), "USD".into(), "9846".into(), "9846.00".into(), "164.10".into()]),
    ];
    let per_lines = lang::fmt_block(&per_rows, per_w);
    assert_eq!(per_lines.len(), 4);
    for l in &per_lines {
        assert!(!l.contains('\t'), "per-product line must not contain tabs: {l:?}");
    }
    // The currency column ("USD") must start at the same byte offset in
    // every row — that only happens if the amount column ({0}) was
    // right-aligned to a common width.
    let eur_offsets: Vec<usize> = per_lines
        .iter()
        .map(|l| l.find("USD").expect("USD present"))
        .collect();
    let first = eur_offsets[0];
    for o in &eur_offsets {
        assert_eq!(*o, first, "USD column must align across all rows:\n{}", per_lines.join("\n"));
    }
    // Likewise the "sales" word must start at the same offset.
    let sales_offsets: Vec<usize> = per_lines
        .iter()
        .map(|l| l.find("sales").or_else(|| l.find("ventas")).expect("sales word present"))
        .collect();
    let first = sales_offsets[0];
    for o in &sales_offsets {
        assert_eq!(*o, first, "sales column must align across all rows:\n{}", per_lines.join("\n"));
    }

    // Totals lines: every numeric column must right-align.
    let tot_w = [d.total_monthly_sales, d.total_annual_sales]
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;
    let totals_rows: Vec<(&'static str, Vec<String>)> = vec![
        (d.total_monthly_sales, vec!["517".into(), "781.00".into(), "13.02".into(), "3".into(), "10".into(), "1.30".into()]),
        (d.total_annual_sales,  vec!["10314".into(), "15462.00".into(), "257.70".into(), "3".into(), "10".into(), "25.77".into()]),
    ];
    let tot_lines = lang::fmt_block(&totals_rows, tot_w);
    for l in &tot_lines {
        assert!(!l.contains('\t'), "totals line must not contain tabs: {l:?}");
    }
    // The 🕐 emoji must start at the same offset in both totals rows.
    let clock_offsets: Vec<usize> = tot_lines
        .iter()
        .map(|l| l.find('🕐').expect("clock present"))
        .collect();
    assert_eq!(
        clock_offsets[0], clock_offsets[1],
        "🕐 column must align across totals rows:\n{}", tot_lines.join("\n")
    );
}

// ---------------------------------------------------------------------------
// write_result_file_monthly tests
// ---------------------------------------------------------------------------

#[test]
fn write_result_file_monthly_in_all_languages() {
    let dir = env::temp_dir().join("sim_test_monthly_all_langs");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let product = ProductDefinition {
        name: "Coffee".into(),
        sale_price: 4.5,
        sale_currency: Currency::new("USD"),
        production_time: 5.0,
        production_time_unit: TimeUnit::Mins,
        costs: vec![Cost {
            price: 1.15,
            currency: Currency::new("USD"),
            description: "beans".into(),
        }],
    };
    let result = compute_result(&product);

    for lang in [Lang::En, Lang::Es, Lang::Zh, Lang::De, Lang::Ru, Lang::Fr] {
        let file = dir.join(format!("prod_{}.txt", lang.code()));
        let goals = [100.0f64; 12];
        let sales = [10i64; 12];
        let minutes = [50.0f64; 12];
        write_result_file_monthly(
            &file, &result, &goals, &sales, &minutes,
            1200.0, 120, 600.0, 8, 2, &lang,
        )
        .unwrap();
        let written = fs::read_to_string(result_file_path(&file)).unwrap();
        // The product name and sale price appear in every language.
        assert!(written.contains("Coffee"), "{} missing product name", lang.code());
        assert!(written.contains("4.50"), "{} missing sale price", lang.code());
        // All 12 localized month abbreviations appear.
        for m in lang.months_abbr().iter() {
            assert!(
                written.contains(m),
                "{} missing month {}",
                lang.code(), m
            );
        }
        // The annual row is present.
        let annual_anchor = match lang {
            Lang::En => "Annual goal:",
            Lang::Es => "Meta anual:",
            Lang::Zh => "年度目标:",
            Lang::De => "Jahresziel:",
            Lang::Ru => "Годовая цель:",
            Lang::Fr => "Objectif annuel :",
        };
        assert!(
            written.contains(annual_anchor),
            "{} missing annual goal row", lang.code()
        );
        // Workday and parallel settings are present.
        assert!(written.contains("8"), "{} missing workday hours", lang.code());
        assert!(written.contains("2"), "{} missing parallel", lang.code());
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn write_totals_file_monthly_in_all_languages() {
    let dir = env::temp_dir().join("sim_test_totals_monthly_all_langs");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let sales = [10i64; 12];
    let minutes = [50.0f64; 12];

    for lang in [Lang::En, Lang::Es, Lang::Zh, Lang::De, Lang::Ru, Lang::Fr] {
        let sub = dir.join(lang.code());
        fs::create_dir_all(&sub).unwrap();
        write_totals_file_monthly(
            &sub, 2, &sales, &minutes, 120, 600.0, 8, 2, &lang,
        )
        .unwrap();
        let written = fs::read_to_string(totals_file_path(&sub)).unwrap();
        // Totals header appears in the localized form.
        let header_fragment = match lang {
            Lang::En => "Totals (2 products)",
            Lang::Es => "Totales (2 productos)",
            Lang::Zh => "合计（2 个产品）",
            Lang::De => "Gesamtergebnis (2 Produkte)",
            Lang::Ru => "Итоги (2 продуктов)",
            Lang::Fr => "Totaux (2 produits)",
        };
        assert!(
            written.contains(header_fragment),
            "{} missing totals header, was:\n{}", lang.code(), written
        );
        // All 12 month abbreviations appear.
        for m in lang.months_abbr().iter() {
            assert!(
                written.contains(m),
                "{} missing month {} in totals", lang.code(), m
            );
        }
        // The totals file is excluded by collect_txt_files.
        let collected = collect_txt_files(&sub);
        assert!(
            !collected.iter().any(|p| p == &totals_file_path(&sub)),
            "{} totals file should not be collected as product", lang.code()
        );
    }

    let _ = fs::remove_dir_all(&dir);
}
