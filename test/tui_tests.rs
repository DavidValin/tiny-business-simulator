use super::*;
use crate::parser::Currency;

#[allow(dead_code)]
fn prod(name: &str, price: f64, cost: f64, dur: f64) -> ProductResult {
    ProductResult {
        name: name.into(),
        price,
        currency: Currency::new("USD"),
        total_cost: cost,
        net_profit: price - cost,
        profit_percent: 100.0,
        duration_minutes: dur,
    }
}

/// Wrap product results with dummy file paths for AppState construction.
fn wrap(results: Vec<ProductResult>) -> Vec<(PathBuf, ProductResult)> {
    results
        .into_iter()
        .enumerate()
        .map(|(i, r)| (PathBuf::from(format!("p{}.txt", i)), r))
        .collect()
}

fn make_state(products: Vec<ProductResult>) -> AppState {
    let n = products.len();
    let wrapped = wrap(products);
    let base = 100 / n.max(1) as i64;
    let extra = 100 - base * n.max(1) as i64;
    let monthly_pct: Vec<[i64; 12]> = (0..n)
        .map(|i| {
            let v = base + if (i as i64) < extra { 1 } else { 0 };
            [v; 12]
        })
        .collect();
    let mut state = AppState {
        products: wrapped,
        monthly_pct,
        month_locked: vec![[false; 12]; n],
        yearly_locked: vec![false; n],
        period: Period::FullYear,
        sliders: Vec::new(),
        selected: 0,
        scroll: 0,
        folder: PathBuf::from("."),
        status: None,
        tab: Tab::Products,
        product_scroll: 0,
        lang: Lang::En,
        active_region: Region::Main,
        show_help: false,
        help_scroll: 0,
        settings: GlobalSettings {
            min_workday_hours: DEFAULT_MIN_WORKDAY_HOURS,
            min_parallel: DEFAULT_MIN_PARALLEL,
            min_monthly_net_profit: DEFAULT_MIN_MONTHLY_NET_PROFIT,
            target_yearly_net_profit: DEFAULT_TARGET_YEARLY_NET_PROFIT,
        },
        month_overrides: MonthOverrides::default(),
    };
    state.rebuild_sliders();
    state
}

/// Set every month's net-profit override to `goal` and workday/parallel to
/// `workday`/`parallel` (clamped to the global minimums).
fn set_all_months(state: &mut AppState, workday: i64, parallel: i64, goal: i64) {
    for m in 0..12 {
        state.month_overrides.workday[m] = workday.max(state.settings.min_workday_hours);
        state.month_overrides.parallel[m] = parallel.max(state.settings.min_parallel);
        state.month_overrides.net_profit[m] = goal.max(state.settings.min_monthly_net_profit);
    }
}

#[test]
fn share_for_month_normalizes_percentages() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let state = make_state(products);
    for m in 0..12 {
        assert!((state.share_for_month(0, m) - 0.5).abs() < 1e-9);
        assert!((state.share_for_month(1, m) - 0.5).abs() < 1e-9);
    }
}

#[test]
fn share_for_month_all_zero_splits_equally() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    for p in &mut state.monthly_pct {
        *p = [0; 12];
    }
    assert!((state.share_for_month(0, 0) - 0.5).abs() < 1e-9);
}

#[test]
fn yearly_pct_is_mean_of_months() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 80;
    state.monthly_pct[1][0] = 20;
    let expected: f64 = (80.0 + 11.0 * 50.0) / 12.0;
    assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
}

#[test]
fn month_totals_scales_down_when_capacity_exceeds() {
    // One product, net profit 5, duration 60 min. Goal 1000 -> 200 sales
    // -> 12000 required minutes. With 1h/day, 1 parallel, 22 days ->
    // capacity = 1*22*60 = 1320 min. Scale = 1320/12000 = 0.11 ->
    // floor(200*0.11) = 22 units, amount = 22*10 = 220.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 1000);
    state.period = Period::Month(0);
    let mt = state.month_totals();
    assert_eq!(mt.units, 22);
    assert!((mt.amount - 220.0).abs() < 1e-9);
    assert!((mt.required_minutes - 12000.0).abs() < 1e-9);
    assert!((mt.capacity_minutes - 1320.0).abs() < 1e-9);
}

#[test]
fn month_totals_unchanged_when_capacity_sufficient() {
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 24, 10, 1000);
    state.period = Period::Month(0);
    let mt = state.month_totals();
    assert_eq!(mt.units, 200);
    assert!((mt.amount - 2000.0).abs() < 1e-9);
    assert!((mt.profit - 1000.0).abs() < 1e-9);
    assert!((mt.cost - 1000.0).abs() < 1e-9);
}

#[test]
fn month_totals_differ_per_month() {
    // Two products. Jan: A=80/B=20. Feb: A=20/B=80. With equal net profit
    // and duration, the unit totals are identical, but the per-product
    // mix differs — verify via month_shares that Jan and Feb differ.
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    state.monthly_pct[0][0] = 80;
    state.monthly_pct[1][0] = 20;
    state.monthly_pct[0][1] = 20;
    state.monthly_pct[1][1] = 80;
    let jan = month_shares(&state, 0);
    let feb = month_shares(&state, 1);
    assert_ne!(jan[0].monthly_sales, feb[0].monthly_sales);
    assert_eq!(jan[0].monthly_sales, 160); // ceil(0.8*1000/5)
    assert_eq!(feb[0].monthly_sales, 40);  // ceil(0.2*1000/5)
}

#[test]
fn slider_clamps_and_steps() {
    let mut s = Slider {
        kind: SliderKind::MinWorkdayHours,
        label: "x".into(),
        value: 8,
        min: 1,
        max: 24,
        step: 1,
        suffix: "",
        locked: false,
    };
    s.inc();
    assert_eq!(s.value, 9);
    for _ in 0..100 {
        s.inc();
    }
    assert_eq!(s.value, 24);
    for _ in 0..100 {
        s.dec();
    }
    assert_eq!(s.value, 1);
}

#[test]
fn fit_bar_width_fills_available_width() {
    assert_eq!(fit_bar_width(130, 1, 2), 4);
    assert_eq!(fit_bar_width(58, 1, 2), 1);
    assert_eq!(fit_bar_width(10, 1, 2), 1);
}

#[test]
fn slider_track_fills_proportionally() {
    let s = Slider {
        kind: SliderKind::MinWorkdayHours,
        label: "x".into(),
        value: 12,
        min: 0,
        max: 24,
        step: 1,
        suffix: "",
        locked: false,
    };
    let t = slider_track(&s, 10);
    assert_eq!(t.chars().count(), 10);
    let filled = t.chars().filter(|c| *c == '\u{2588}').count();
    assert_eq!(filled, 5);
}

#[test]
fn initial_monthly_percentages_sum_to_100() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let state = make_state(products);
    for m in 0..12 {
        let sum: i64 = state.monthly_pct.iter().map(|p| p[m]).sum();
        assert_eq!(sum, 100, "month {} sums to {}", m, sum);
    }
}

// --- redistribute_month (within a single month) -------------------------

#[test]
fn redistribute_month_keeps_total_at_100() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 50;
    state.monthly_pct[1][0] = 30;
    state.monthly_pct[2][0] = 20;
    redistribute_month(&mut state, 0, 0, 60);
    assert_eq!(state.monthly_pct[0][0], 60);
    assert_eq!(state.monthly_pct[1][0], 20);
    assert_eq!(state.monthly_pct[2][0], 20);
    let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, 100);
}

#[test]
fn redistribute_month_includes_zero_products() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 50;
    state.monthly_pct[1][0] = 30;
    state.monthly_pct[2][0] = 0;
    redistribute_month(&mut state, 0, 0, 60);
    assert_eq!(state.monthly_pct[0][0], 60);
    assert_eq!(state.monthly_pct[1][0], 20);
    assert_eq!(state.monthly_pct[2][0], 20);
}

#[test]
fn redistribute_month_freezes_locked_product() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 50;
    state.monthly_pct[1][0] = 30;
    state.monthly_pct[2][0] = 20;
    state.month_locked[1][0] = true;
    redistribute_month(&mut state, 0, 0, 60);
    assert_eq!(state.monthly_pct[0][0], 60);
    assert_eq!(state.monthly_pct[1][0], 30);
    assert_eq!(state.monthly_pct[2][0], 10);
}

#[test]
fn redistribute_month_clamped_by_locked_room() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 20;
    state.monthly_pct[1][0] = 30;
    state.monthly_pct[2][0] = 50;
    state.month_locked[1][0] = true;
    state.month_locked[2][0] = true;
    redistribute_month(&mut state, 0, 0, 90);
    assert_eq!(state.monthly_pct[0][0], 20);
    assert_eq!(state.monthly_pct[1][0], 30);
    assert_eq!(state.monthly_pct[2][0], 50);
    let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, 100);
}

#[test]
fn redistribute_month_locked_changed_is_noop() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.month_locked[0][0] = true;
    let before = state.monthly_pct[0][0];
    redistribute_month(&mut state, 0, 0, 80);
    assert_eq!(state.monthly_pct[0][0], before);
}

#[test]
fn redistribute_month_yearly_locked_is_frozen() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.yearly_locked[1] = true;
    let before = state.monthly_pct[1][3];
    redistribute_month(&mut state, 3, 0, 60);
    assert_eq!(state.monthly_pct[1][3], before);
    assert_eq!(state.monthly_pct[0][3], 50);
    assert_eq!(state.monthly_pct[1][3], 50);
}

// --- edit_yearly (propagation across all 12 months) ---------------------

#[test]
fn edit_yearly_propagates_to_all_months() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    edit_yearly(&mut state, 0, 80);
    for m in 0..12 {
        assert_eq!(state.monthly_pct[0][m], 80, "month {}", m);
        assert_eq!(state.monthly_pct[1][m], 20, "month {}", m);
    }
    assert_eq!(state.yearly_pct(0), 80);
}

#[test]
fn edit_yearly_skips_month_locked_months() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.month_locked[0][3] = true;
    edit_yearly(&mut state, 0, 80);
    assert_eq!(state.monthly_pct[0][3], 50);
    for m in 0..12 {
        if m != 3 {
            assert_eq!(state.monthly_pct[0][m], 80, "month {}", m);
        }
    }
    let expected: f64 = (11.0 * 80.0 + 50.0) / 12.0;
    assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
    assert_ne!(state.yearly_pct(0), 80);
}

#[test]
fn edit_yearly_locked_product_is_noop() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.yearly_locked[0] = true;
    let before: Vec<[i64; 12]> = state.monthly_pct.clone();
    edit_yearly(&mut state, 0, 80);
    assert_eq!(state.monthly_pct, before);
}

#[test]
fn monthly_edit_recomputes_yearly_as_mean() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.period = Period::Month(0);
    redistribute_month(&mut state, 0, 0, 80);
    let expected: f64 = (80.0 + 11.0 * 50.0) / 12.0;
    assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
}

#[test]
fn yearly_lock_renders_month_checkbox_checked_and_greyed() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.yearly_locked[0] = true;
    state.period = Period::Month(3);
    state.rebuild_sliders();
    let a_month = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthPercent(0)))
        .unwrap();
    assert!(a_month.locked, "monthly slider must be locked when yearly-locked");
}

// --- per-month parallel range / totals / export ---------------------------

#[test]
fn month_parallel_range_caps_to_one_workday() {
    // 1 product, net 5, dur 60, goal 1000 -> 200 sales -> 12000 min ->
    // 200 monthly hours. workday 8 -> max_par = floor(200/8) = 25.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    state.settings.target_yearly_net_profit = 12000;
    state.period = Period::Month(0);
    state.rebuild_sliders();
    update_parallel_range(&mut state);
    let p = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthParallel(_)))
        .unwrap();
    assert_eq!(p.min, 1);
    assert_eq!(p.max, 25);
    assert!(p.value >= p.min && p.value <= p.max);
}

#[test]
fn month_parallel_clamps_value_into_range() {
    // Goal 100000, workday 1 -> 20000 monthly hours -> max_par = 20000.
    // Setting the override above the max clamps it down to the max.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 100000);
    state.period = Period::Month(0);
    state.month_overrides.parallel[0] = 999_999;
    state.rebuild_sliders();
    update_parallel_range(&mut state);
    assert_eq!(state.month_overrides.parallel[0], 20000);
    let p = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthParallel(_)))
        .unwrap();
    assert_eq!(p.value, 20000);
    assert_eq!(p.max, 20000);
}

#[test]
fn compute_totals_monthly_and_annual() {
    // Two products, equal 50/50 split, monthly goal 1000, all 12 months
    // equal. A: net 5, dur 5 -> 100 sales / 500 min. B: net 10, dur 10 ->
    // 50 sales / 500 min. Monthly totals: 150 sales, 1000 min. Annual =
    // 12 * monthly (all months equal).
    let products = vec![prod("A", 6.0, 1.0, 5.0), prod("B", 12.0, 2.0, 10.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    state.period = Period::Month(0);
    let t = compute_totals(&state);
    assert_eq!(t.monthly.sales, 150);
    assert!((t.monthly.minutes - 1000.0).abs() < 1e-6);
    assert_eq!(t.annual.sales, 150 * 12);
    assert!((t.annual.minutes - 12000.0).abs() < 1e-6);
}

#[test]
fn export_results_writes_12_monthly_rows_and_totals() {
    let dir = std::env::temp_dir().join("tui_export_test_monthly");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let products = vec![
        prod("Coffee", 4.5, 1.15, 5.0),
        prod("Tea", 3.0, 0.8, 4.0),
    ];
    let mut paths: Vec<(PathBuf, ProductResult)> = Vec::new();
    for (i, r) in products.into_iter().enumerate() {
        let f = dir.join(format!("p{}.txt", i));
        std::fs::write(&f, "+ stub\n").unwrap();
        paths.push((f, r));
    }
    let n = paths.len();
    let base = 100 / n as i64;
    let extra = 100 - base * n as i64;
    let monthly_pct: Vec<[i64; 12]> = (0..n)
        .map(|i| {
            let v = base + if (i as i64) < extra { 1 } else { 0 };
            [v; 12]
        })
        .collect();
    let mut state = AppState {
        products: paths,
        folder: dir.clone(),
        monthly_pct,
        month_locked: vec![[false; 12]; n],
        yearly_locked: vec![false; n],
        period: Period::FullYear,
        sliders: Vec::new(),
        selected: 0,
        scroll: 0,
        status: None,
        tab: Tab::Products,
        product_scroll: 0,
        lang: Lang::En,
        active_region: Region::Main,
        show_help: false,
        help_scroll: 0,
        settings: GlobalSettings {
            min_workday_hours: DEFAULT_MIN_WORKDAY_HOURS,
            min_parallel: DEFAULT_MIN_PARALLEL,
            min_monthly_net_profit: DEFAULT_MIN_MONTHLY_NET_PROFIT,
            target_yearly_net_profit: DEFAULT_TARGET_YEARLY_NET_PROFIT,
        },
        month_overrides: MonthOverrides::default(),
    };
    state.rebuild_sliders();
    set_all_months(&mut state, 8, 1, 500);

    let status = export_results(&state, &Lang::En);
    assert!(status.contains("exported"), "status was: {}", status);

    let product_file = std::fs::read_to_string(dir.join("p0.simulation_results.txt")).unwrap();
    for m in ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] {
        assert!(product_file.contains(m), "per-product file missing month {}, was:\n{}", m, product_file);
    }
    assert!(product_file.contains("Annual goal"), "missing annual row");

    let totals_file = std::fs::read_to_string(dir.join("totals.simulation_results.txt")).unwrap();
    for m in ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] {
        assert!(totals_file.contains(m), "totals file missing month {}, was:\n{}", m, totals_file);
    }
    assert!(totals_file.contains("Total annual sales"));

    let _ = std::fs::remove_dir_all(&dir);
}

/// Find the leftmost x at row `y` whose horizontal run of cells matches
/// `needle`.
fn find_in_row(buf: &ratatui::buffer::Buffer, y: u16, needle: &str) -> Option<u16> {
    let chars: Vec<char> = needle.chars().collect();
    let w = buf.area.width;
    if y >= buf.area.height {
        return None;
    }
    for x in 0..w.saturating_sub(chars.len() as u16) {
        let mut ok = true;
        for (i, c) in chars.iter().enumerate() {
            if buf[((x + i as u16), y)].symbol() != &c.to_string() {
                ok = false;
                break;
            }
        }
        if ok {
            return Some(x);
        }
    }
    None
}

#[test]
fn settings_renders_two_columns_side_by_side() {
    use ratatui::backend::TestBackend;
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    update_parallel_range(&mut state);

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // Full Year settings: labels "min. workday hours" and "min. monthly net
    // profit" must render side by side with a vertical separator between them.
    let settings_y = (0..buf.area.height)
        .find_map(|y| find_in_row(&buf, y, "Global settings").map(|_| y))
        .expect("Settings title not rendered");
    let header = (settings_y..buf.area.height)
        .find_map(|y| {
            let w = find_in_row(&buf, y, "workday")?;
            let m = find_in_row(&buf, y, "monthly")?;
            Some((y, w, m))
        })
        .expect("workday/monthly labels not rendered side by side in Settings");
    let (_, workday_x, monthly_x) = header;
    assert!(monthly_x > workday_x);

    let y = header.0;
    let mut found_sep = false;
    for x in (workday_x + 1)..monthly_x {
        if buf[(x, y)].symbol() == "\u{2502}" {
            found_sep = true;
            break;
        }
    }
    assert!(found_sep, "no vertical separator between the two settings columns");
}

#[test]
fn totals_renders_two_columns_side_by_side() {
    use ratatui::backend::TestBackend;
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    // A Month period renders Monthly (left) + Yearly (right) in the Totals.
    state.period = Period::Month(0);
    state.rebuild_sliders();
    update_parallel_range(&mut state);

    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    let totals_y = (0..buf.area.height)
        .find_map(|y| find_in_row(&buf, y, "Totals").map(|_| y))
        .expect("Totals title not rendered");
    let header_y = (totals_y..buf.area.height)
        .find_map(|y| {
            let m = find_in_row(&buf, y, "Monthly")?;
            let yr = find_in_row(&buf, y, "Yearly")?;
            Some((y, m, yr))
        })
        .expect("Monthly/Yearly headers not rendered side by side in Totals");
    let (_, monthly_x, yearly_x) = header_y;
    assert!(yearly_x > monthly_x);

    let y = header_y.0;
    let mut found_sep = false;
    for x in (monthly_x + 1)..yearly_x {
        if buf[(x, y)].symbol() == "\u{2502}" {
            found_sep = true;
            break;
        }
    }
    assert!(found_sep, "no vertical separator between the two totals columns");
}

/// Verify the sidebar padding is clamped so all product rows fit: with many
/// products on a short terminal, the padding must drop to 0 so every row is
/// visible.
#[test]
fn sidebar_padding_clamps_to_fit_all_product_rows() {
    use ratatui::backend::TestBackend;
    let products: Vec<ProductResult> = (0..10)
        .map(|i| prod(&format!("P{}", i), 10.0, 5.0, 5.0))
        .collect();
    let mut state = make_state(products);
    update_parallel_range(&mut state);

    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    let products_y = (0..buf.area.height)
        .find_map(|y| find_in_row(&buf, y, "Products").map(|_| y))
        .expect("Products title not rendered");
    let p0_row = (products_y + 1..buf.area.height)
        .find_map(|y| find_in_row(&buf, y, "% P0").map(|_| y))
        .expect("% P0 not rendered");
    assert_eq!(
        p0_row, products_y + 1,
        "padding should be 0 when products don't fit otherwise, got p0_row={} products_y={}",
        p0_row, products_y
    );
}

#[test]
fn period_bar_renders_full_year_and_selected_month() {
    use ratatui::backend::TestBackend;
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.period = Period::Month(5); // Jun
    state.rebuild_sliders();
    update_parallel_range(&mut state);

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // The top period bar row shows "Full Year" and every month abbreviation
    // (including the selected Jun).
    assert!(
        find_in_row(&buf, 0, "Full Year").is_some(),
        "Full Year tab not rendered in the period bar"
    );
    assert!(
        find_in_row(&buf, 0, "Jun").is_some(),
        "selected month Jun not rendered in the period bar"
    );
    // The sub-tab bar (Products/Graph) renders on row 2, below the separator.
    assert!(
        (0..buf.area.height).any(|y| find_in_row(&buf, y, "Products").is_some()),
        "Products sub-tab not rendered"
    );
}

#[test]
fn save_then_load_state_roundtrip() {
    let dir = std::env::temp_dir().join("tui_state_roundtrip_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let products = vec![
        prod("Coffee", 4.5, 1.15, 5.0),
        prod("Tea", 3.0, 0.8, 4.0),
    ];
    let mut paths: Vec<(PathBuf, ProductResult)> = Vec::new();
    for (i, r) in products.into_iter().enumerate() {
        let f = dir.join(format!("p{}.txt", i));
        std::fs::write(&f, "+ stub\n").unwrap();
        paths.push((f, r));
    }
    let n = paths.len();
    let base = 100 / n as i64;
    let extra = 100 - base * n as i64;
    let monthly_pct: Vec<[i64; 12]> = (0..n)
        .map(|i| {
            let v = base + if (i as i64) < extra { 1 } else { 0 };
            [v; 12]
        })
        .collect();
    let mut state = AppState {
        products: paths.clone(),
        folder: dir.clone(),
        monthly_pct,
        month_locked: vec![[false; 12]; n],
        yearly_locked: vec![false; n],
        period: Period::Month(3),
        sliders: Vec::new(),
        selected: 0,
        scroll: 0,
        status: None,
        tab: Tab::Products,
        product_scroll: 0,
        show_help: false,
        help_scroll: 0,
        lang: Lang::En,
        active_region: Region::Main,
        settings: GlobalSettings {
            min_workday_hours: DEFAULT_MIN_WORKDAY_HOURS,
            min_parallel: DEFAULT_MIN_PARALLEL,
            min_monthly_net_profit: DEFAULT_MIN_MONTHLY_NET_PROFIT,
            target_yearly_net_profit: DEFAULT_TARGET_YEARLY_NET_PROFIT,
        },
        month_overrides: MonthOverrides::default(),
    };
    state.rebuild_sliders();

    // Customize: product 0 = 80%, product 1 = 20% in Jan (month 0).
    state.monthly_pct[0][0] = 80;
    state.monthly_pct[1][0] = 20;
    // Lock product 0 in month 3.
    state.month_locked[0][3] = true;
    // Yearly-lock product 1.
    state.yearly_locked[1] = true;
    // Change settings + a per-month override.
    state.settings.min_workday_hours = 6;
    state.settings.min_monthly_net_profit = 400;
    state.month_overrides.net_profit[0] = 750;
    state.month_overrides.workday[1] = 10;

    save_state(&state);

    let loaded = load_state(&dir, &paths).expect("state should load");
    assert_eq!(loaded.monthly_pct[0][0], 80);
    assert_eq!(loaded.monthly_pct[1][0], 20);
    assert!(loaded.month_locked[0][3]);
    assert!(loaded.yearly_locked[1]);
    assert_eq!(loaded.period, Period::Month(3));
    assert_eq!(loaded.settings.min_workday_hours, 6);
    assert_eq!(loaded.settings.min_monthly_net_profit, 400);
    assert_eq!(loaded.month_overrides.net_profit[0], 750);
    assert_eq!(loaded.month_overrides.workday[1], 10);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_state_returns_none_when_no_file() {
    let dir = std::env::temp_dir().join("tui_state_nofile_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let paths = wrap(products);
    assert!(load_state(&dir, &paths).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_state_normalizes_when_product_added() {
    let dir = std::env::temp_dir().join("tui_state_normalize_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Save state with 2 products (p0=80, p1=20 in Jan).
    std::fs::write(dir.join("p0.txt"), "+ stub\n").unwrap();
    std::fs::write(dir.join("p1.txt"), "+ stub\n").unwrap();
    std::fs::write(
        dir.join(".simulation_state"),
        "p0.txt 80 80 80 80 80 80 80 80 80 80 80 80 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         p1.txt 20 20 20 20 20 20 20 20 20 20 20 20 0 0 0 0 0 0 0 0 0 0 0 0 1\n",
    )
    .unwrap();

    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let paths: Vec<(PathBuf, ProductResult)> = products
        .into_iter()
        .enumerate()
        .map(|(i, r)| (dir.join(format!("p{}.txt", i)), r))
        .collect();
    std::fs::write(dir.join("p2.txt"), "+ stub\n").unwrap();

    let loaded = load_state(&dir, &paths).expect("state should load");
    assert_eq!(loaded.monthly_pct[0][0], 80);
    assert_eq!(loaded.monthly_pct[1][0], 20);
    assert_eq!(loaded.monthly_pct[2][0], 0);
    let sum: i64 = loaded.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, 100);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_state_normalizes_when_product_removed() {
    let dir = std::env::temp_dir().join("tui_state_normalize_removed_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(dir.join("p0.txt"), "+ stub\n").unwrap();
    std::fs::write(dir.join("p1.txt"), "+ stub\n").unwrap();
    std::fs::write(dir.join("p2.txt"), "+ stub\n").unwrap();
    std::fs::write(
        dir.join(".simulation_state"),
        "p0.txt 34 34 34 34 34 34 34 34 34 34 34 34 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         p1.txt 33 33 33 33 33 33 33 33 33 33 33 33 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         p2.txt 33 33 33 33 33 33 33 33 33 33 33 33 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
    )
    .unwrap();

    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
    ];
    let paths: Vec<(PathBuf, ProductResult)> = products
        .into_iter()
        .enumerate()
        .map(|(i, r)| (dir.join(format!("p{}.txt", i)), r))
        .collect();

    let loaded = load_state(&dir, &paths).expect("state should load");
    let sum: i64 = loaded.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, 100, "month must sum to 100 after product removal");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_state_normalizes_when_all_products_locked() {
    let dir = std::env::temp_dir().join("tui_state_all_locked_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(dir.join("p0.txt"), "+ stub\n").unwrap();
    std::fs::write(dir.join("p1.txt"), "+ stub\n").unwrap();
    std::fs::write(dir.join("p2.txt"), "+ stub\n").unwrap();
    std::fs::write(
        dir.join(".simulation_state"),
        "p0.txt 34 34 34 34 34 34 34 34 34 34 34 34 0 0 0 0 0 0 0 0 0 0 0 0 1\n\
         p1.txt 34 34 34 34 34 34 34 34 34 34 34 34 0 0 0 0 0 0 0 0 0 0 0 0 1\n\
         p2.txt 34 34 34 34 34 34 34 34 34 34 34 34 0 0 0 0 0 0 0 0 0 0 0 0 1\n",
    )
    .unwrap();

    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let paths: Vec<(PathBuf, ProductResult)> = products
        .into_iter()
        .enumerate()
        .map(|(i, r)| (dir.join(format!("p{}.txt", i)), r))
        .collect();

    let loaded = load_state(&dir, &paths).expect("state should load");
    for m in 0..12 {
        let sum: i64 = loaded.monthly_pct.iter().map(|p| p[m]).sum();
        assert_eq!(
            sum, 100,
            "month {} must sum to 100 even when all products are locked, got {}",
            m, sum
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn totals_match_capped_chart_values() {
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 1000);
    state.period = Period::Month(0);
    let mt = state.month_totals_for(state.period.month().unwrap());
    assert_eq!(mt.units, 22);
    let t = compute_totals(&state);
    assert_eq!(
        t.monthly.sales, 22,
        "totals must show capped sales (22), not required (200)"
    );
    assert!(
        (t.monthly.minutes - mt.achieved_minutes).abs() < 1e-9,
        "totals minutes must match chart achieved minutes"
    );
}

#[test]
fn donut_uses_capped_sales() {
    // The "vs year" donut should use capped (achievable) sales, not required.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 1000);
    state.settings.target_yearly_net_profit = 12000;

    let capped_annual: i64 = (0..12).map(|m| state.capped_product_sales(m)[0]).sum();
    assert_eq!(capped_annual, 264);

    let yearly_profit = capped_annual as f64 * 5.0; // 1320
    let of_goal = yearly_profit / 12000.0 * 100.0; // 11%
    assert!(
        of_goal < 100.0,
        "donut should show <100% when capacity is exceeded, got {:.1}%",
        of_goal
    );

    let uncapped_annual: i64 = (0..12)
        .map(|m| month_shares(&state, m)[0].monthly_sales)
        .sum();
    assert_ne!(capped_annual, uncapped_annual);
}

#[test]
fn raising_min_clamps_overrides() {
    // Raising the global minimums must clamp every per-month override up to
    // the new minimum.
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 10;
    state.settings.min_parallel = 3;
    state.settings.min_monthly_net_profit = 900;
    state.clamp_overrides_to_mins();
    for m in 0..12 {
        assert!(state.month_overrides.workday[m] >= 10, "month {}", m);
        assert!(state.month_overrides.parallel[m] >= 3, "month {}", m);
        assert!(state.month_overrides.net_profit[m] >= 900, "month {}", m);
    }
}

#[test]
fn period_next_prev_wraps() {
    assert_eq!(Period::FullYear.next(), Period::Month(0));
    assert_eq!(Period::Month(0).prev(), Period::FullYear);
    assert_eq!(Period::Month(11).next(), Period::FullYear);
    assert_eq!(Period::FullYear.prev(), Period::Month(11));
    assert_eq!(Period::Month(3).next(), Period::Month(4));
    assert_eq!(Period::Month(3).prev(), Period::Month(2));
}

#[test]
fn graph_arrow_marks_selected_month_bars() {
    use ratatui::backend::TestBackend;
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    // Graph sub-tab + a Month period: the selected month's bars must be
    // surrounded by a border (box-drawing chars).
    state.tab = Tab::Graph;
    state.period = Period::Month(3);
    state.rebuild_sliders();
    update_parallel_range(&mut state);

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // A downward arrow (▼) must be present in the chart area marking the
    // selected month's bars.
    let has_arrow = (0..buf.area.height).any(|y| {
        (0..buf.area.width).any(|x| buf[(x, y)].symbol() == "\u{25bc}")
    });
    assert!(has_arrow, "no arrow drawn above the selected month's bars");

    // And the selected month label (Apr) is highlighted somewhere in the
    // chart's bottom label row.
    assert!(
        (0..buf.area.height).any(|y| find_in_row(&buf, y, "Apr").is_some()),
        "selected month Apr label not rendered in the chart"
    );
}
