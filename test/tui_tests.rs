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
    // Percentages are stored in tenths of a percent (PCT_TOTAL == 1000).
    let base = PCT_TOTAL / n.max(1) as i64;
    let extra = PCT_TOTAL - base * n.max(1) as i64;
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
    state.monthly_pct[0][0] = 800;
    state.monthly_pct[1][0] = 200;
    let expected: f64 = (800.0 + 11.0 * 500.0) / 12.0;
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
    state.monthly_pct[0][0] = 800;
    state.monthly_pct[1][0] = 200;
    state.monthly_pct[0][1] = 200;
    state.monthly_pct[1][1] = 800;
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
fn initial_monthly_percentages_sum_to_1000() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let state = make_state(products);
    for m in 0..12 {
        let sum: i64 = state.monthly_pct.iter().map(|p| p[m]).sum();
        assert_eq!(sum, PCT_TOTAL, "month {} sums to {}", m, sum);
    }
}

// --- redistribute_month (within a single month) -------------------------

#[test]
fn redistribute_month_keeps_total_at_1000() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 500;
    state.monthly_pct[1][0] = 300;
    state.monthly_pct[2][0] = 200;
    redistribute_month(&mut state, 0, 0, 600);
    assert_eq!(state.monthly_pct[0][0], 600);
    assert_eq!(state.monthly_pct[1][0], 200);
    assert_eq!(state.monthly_pct[2][0], 200);
    let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, PCT_TOTAL);
}

#[test]
fn redistribute_month_includes_zero_products() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 500;
    state.monthly_pct[1][0] = 300;
    state.monthly_pct[2][0] = 0;
    redistribute_month(&mut state, 0, 0, 600);
    assert_eq!(state.monthly_pct[0][0], 600);
    assert_eq!(state.monthly_pct[1][0], 200);
    assert_eq!(state.monthly_pct[2][0], 200);
}

#[test]
fn redistribute_month_freezes_locked_product() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 500;
    state.monthly_pct[1][0] = 300;
    state.monthly_pct[2][0] = 200;
    state.month_locked[1][0] = true;
    redistribute_month(&mut state, 0, 0, 600);
    assert_eq!(state.monthly_pct[0][0], 600);
    assert_eq!(state.monthly_pct[1][0], 300);
    assert_eq!(state.monthly_pct[2][0], 100);
}

#[test]
fn redistribute_month_clamped_by_locked_room() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 200;
    state.monthly_pct[1][0] = 300;
    state.monthly_pct[2][0] = 500;
    state.month_locked[1][0] = true;
    state.month_locked[2][0] = true;
    redistribute_month(&mut state, 0, 0, 900);
    assert_eq!(state.monthly_pct[0][0], 200);
    assert_eq!(state.monthly_pct[1][0], 300);
    assert_eq!(state.monthly_pct[2][0], 500);
    let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, PCT_TOTAL);
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
    redistribute_month(&mut state, 3, 0, 600);
    assert_eq!(state.monthly_pct[1][3], before);
    assert_eq!(state.monthly_pct[0][3], 500);
    assert_eq!(state.monthly_pct[1][3], 500);
}

// --- edit_yearly (propagation across all 12 months) ---------------------

#[test]
fn edit_yearly_propagates_to_all_months() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    edit_yearly(&mut state, 0, 800);
    for m in 0..12 {
        assert_eq!(state.monthly_pct[0][m], 800, "month {}", m);
        assert_eq!(state.monthly_pct[1][m], 200, "month {}", m);
    }
    assert_eq!(state.yearly_pct(0), 800);
}

#[test]
fn edit_yearly_skips_month_locked_months() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.month_locked[0][3] = true;
    edit_yearly(&mut state, 0, 800);
    assert_eq!(state.monthly_pct[0][3], 500);
    for m in 0..12 {
        if m != 3 {
            assert_eq!(state.monthly_pct[0][m], 800, "month {}", m);
        }
    }
    let expected: f64 = (11.0 * 800.0 + 500.0) / 12.0;
    assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
    assert_ne!(state.yearly_pct(0), 800);
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
    redistribute_month(&mut state, 0, 0, 800);
    let expected: f64 = (800.0 + 11.0 * 500.0) / 12.0;
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
    // 200 monthly hours. The cap uses a fixed 1-hour reference (independent
    // of workday hours), so max_par = floor(200) = 200.
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
    assert_eq!(p.max, 200);
    assert!(p.value >= p.min && p.value <= p.max);
}

#[test]
fn month_parallel_range_decoupled_from_workday_hours() {
    // Raising workday hours must NOT shrink the parallel cap or clamp the
    // stored parallel value down — both sliders independently grow capacity.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 50, 1000);
    state.period = Period::Month(0);
    state.rebuild_sliders();
    update_parallel_range(&mut state);
    let max_at_8h = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthParallel(_)))
        .unwrap()
        .max;
    let par_at_8h = state.month_overrides.parallel[0];

    // Double workday hours; the cap and stored parallel must be unchanged.
    state.month_overrides.workday[0] = 16;
    state.rebuild_sliders();
    update_parallel_range(&mut state);
    let p = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthParallel(_)))
        .unwrap();
    assert_eq!(p.max, max_at_8h, "parallel cap shrank when workday hours rose");
    assert_eq!(
        state.month_overrides.parallel[0], par_at_8h,
        "parallel value was clamped down when workday hours rose"
    );
}

#[test]
fn month_parallel_clamps_value_into_range() {
    // Goal 100000 -> 20000 sales -> 1,200,000 min -> 20000 monthly hours.
    // max_par uses the 1-hour reference = floor(20000) = 20000. Setting the
    // override above the max clamps it down to the max.
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
    let base = PCT_TOTAL / n as i64;
    let extra = PCT_TOTAL - base * n as i64;
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
    let base = PCT_TOTAL / n as i64;
    let extra = PCT_TOTAL - base * n as i64;
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
    state.monthly_pct[0][0] = 800;
    state.monthly_pct[1][0] = 200;
    // Lock product 0 in month 3.
    state.month_locked[0][3] = true;
    // Yearly-lock product 1.
    state.yearly_locked[1] = true;
    // Change settings + a per-month override.
    state.settings.min_workday_hours = 6;
    state.settings.min_monthly_net_profit = 400;
    state.month_overrides.net_profit[0] = 750;
    state.month_overrides.workday[1] = 10;
    state.month_overrides.fix_costs[2] = 300;

    save_state(&state);

    let loaded = load_state(&dir, &paths).expect("state should load");
    assert_eq!(loaded.monthly_pct[0][0], 800);
    assert_eq!(loaded.monthly_pct[1][0], 200);
    assert!(loaded.month_locked[0][3]);
    assert!(loaded.yearly_locked[1]);
    assert_eq!(loaded.period, Period::Month(3));
    assert_eq!(loaded.settings.min_workday_hours, 6);
    assert_eq!(loaded.settings.min_monthly_net_profit, 400);
    assert_eq!(loaded.month_overrides.net_profit[0], 750);
    assert_eq!(loaded.month_overrides.workday[1], 10);
    assert_eq!(loaded.month_overrides.fix_costs[2], 300);

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

    // Save state with 2 products (p0=80%, p1=20% in Jan).
    std::fs::write(dir.join("p0.txt"), "+ stub\n").unwrap();
    std::fs::write(dir.join("p1.txt"), "+ stub\n").unwrap();
    std::fs::write(
        dir.join("simulation_state.txt"),
        "p0.txt 800 800 800 800 800 800 800 800 800 800 800 800 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         p1.txt 200 200 200 200 200 200 200 200 200 200 200 200 0 0 0 0 0 0 0 0 0 0 0 0 1\n",
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
    assert_eq!(loaded.monthly_pct[0][0], 800);
    assert_eq!(loaded.monthly_pct[1][0], 200);
    assert_eq!(loaded.monthly_pct[2][0], 0);
    let sum: i64 = loaded.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, PCT_TOTAL);

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
        dir.join("simulation_state.txt"),
        "p0.txt 340 340 340 340 340 340 340 340 340 340 340 340 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         p1.txt 330 330 330 330 330 330 330 330 330 330 330 330 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         p2.txt 330 330 330 330 330 330 330 330 330 330 330 330 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
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
    assert_eq!(sum, PCT_TOTAL, "month must sum to 1000 after product removal");

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
        dir.join("simulation_state.txt"),
        "p0.txt 340 340 340 340 340 340 340 340 340 340 340 340 0 0 0 0 0 0 0 0 0 0 0 0 1\n\
         p1.txt 330 330 330 330 330 330 330 330 330 330 330 330 0 0 0 0 0 0 0 0 0 0 0 0 1\n\
         p2.txt 330 330 330 330 330 330 330 330 330 330 330 330 0 0 0 0 0 0 0 0 0 0 0 0 1\n",
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
            sum, PCT_TOTAL,
            "month {} must sum to 1000 even when all products are locked, got {}",
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

#[test]
fn compact_value_formats_small_numbers_as_is() {
    assert_eq!(compact_value(0.0), "0");
    assert_eq!(compact_value(42.0), "42");
    assert_eq!(compact_value(999.0), "999");
}

#[test]
fn compact_value_uses_k_suffix_for_thousands() {
    assert_eq!(compact_value(1000.0), "1K");
    assert_eq!(compact_value(1500.0), "2K");
    assert_eq!(compact_value(9999.0), "10K");
    assert_eq!(compact_value(14500.0), "15K");
}

#[test]
fn compact_value_uses_m_suffix_for_millions() {
    assert_eq!(compact_value(1_000_000.0), "1.0M");
    assert_eq!(compact_value(1_500_000.0), "1.5M");
}

#[test]
fn compact_value_does_not_decrease_when_value_grows() {
    // Regression: when a value crosses a power of 10, the old truncation
    // made the displayed value shrink (e.g. 8800 → "88", 14500 → "14").
    // With compact_value the displayed order of magnitude never goes backwards.
    let a = compact_value(8800.0);   // "9K"
    let b = compact_value(14500.0);  // "15K"
    assert!(
        b.ends_with('K') && a.ends_with('K'),
        "both should use K suffix"
    );
    let parse_k = |s: &str| s.trim_end_matches('K').parse::<f64>().unwrap();
    assert!(
        parse_k(&b) > parse_k(&a),
        "compact_value decreased when value grew: {a} -> {b}"
    );
}

// ---------------------------------------------------------------------------
// Multi-product capacity capping
// ---------------------------------------------------------------------------

#[test]
fn compute_month_scales_multiple_products_proportionally() {
    // Two products with different durations. When capacity is exceeded,
    // both are scaled by the same factor so total achieved minutes fit.
    let products = vec![
        prod("A", 10.0, 5.0, 10.0), // net 5, dur 10 min
        prod("B", 10.0, 5.0, 20.0), // net 5, dur 20 min
    ];
    let mut state = make_state(products);
    // Goal 1000, 50/50 split -> each: ceil(500/5)=100 sales.
    // Required minutes: A=100*10=1000, B=100*20=2000, total=3000.
    // Capacity with 1h/day, 1 parallel: 1*22*60=1320.
    // Scale = 1320/3000 = 0.44.
    // Capped: A=floor(100*0.44)=44, B=floor(100*0.44)=44.
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 1000);
    state.period = Period::Month(0);
    let mt = state.month_totals();
    assert_eq!(mt.units, 88); // 44 + 44
    // Achieved minutes: 44*10 + 44*20 = 440 + 880 = 1320 = capacity
    assert!((mt.achieved_minutes - 1320.0).abs() < 1e-9);
    assert!((mt.capacity_minutes - 1320.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// Annual workdays summed per-month (each using its own override)
// ---------------------------------------------------------------------------

#[test]
fn compute_totals_annual_workdays_sum_per_month() {
    // Each month has different workday hours, so annual workdays must be
    // the sum of per-month workdays (each using its own override), not a
    // single global value.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    // Month 0: 16h/day (more capacity -> not capped, 200 units).
    // Months 1..11: 8h/day (capped: 176 units).
    state.month_overrides.workday[0] = 16;

    // Month 0: capacity = 16*22*60 = 21120 > 12000 (required) -> 200 units.
    //   achieved = 12000, hours = 200, workdays = 200/16 = 12.5.
    // Months 1..11: capacity = 8*22*60 = 10560 < 12000 -> scale = 0.88.
    //   capped = floor(200*0.88) = 176, achieved = 10560, hours = 176,
    //   workdays = 176/8 = 22.
    let t = compute_totals(&state);
    let expected = 12.5 + 22.0 * 11.0;
    assert!(
        (t.annual.workdays - expected).abs() < 1e-6,
        "annual workdays = {}, expected {}",
        t.annual.workdays, expected
    );
}

#[test]
fn compute_totals_with_varying_monthly_goals() {
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 500);
    // Month 0: goal 2000 (not capped, capacity 10560 > 2000).
    // Months 1..11: goal 500 (not capped).
    state.month_overrides.net_profit[0] = 2000;
    // Month 0: ceil(2000/5)=400 sales, 400*5=2000 min.
    // Months 1..11: ceil(500/5)=100 sales, 100*5=500 min.
    let t = compute_totals(&state);
    assert_eq!(t.annual.sales, 400 + 100 * 11);
    assert!((t.annual.minutes - (2000.0 + 500.0 * 11.0)).abs() < 1e-6);
}

// ---------------------------------------------------------------------------
// Goal-achievement indicator logic
// ---------------------------------------------------------------------------

#[test]
fn monthly_profit_meets_goal_when_capacity_sufficient() {
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    state.period = Period::Month(0);
    let t = compute_totals(&state);
    // Goal 1000, net 5 -> 200 sales -> profit = 200*5 = 1000 = goal.
    assert!((t.monthly.profit - 1000.0).abs() < 1e-9);
    assert!(t.monthly.profit >= state.monthly_goal(0) as f64);
}

#[test]
fn monthly_profit_does_not_meet_goal_when_capped() {
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 1000);
    state.period = Period::Month(0);
    let t = compute_totals(&state);
    // Goal 1000 but capacity-limited -> profit < 1000.
    assert!(
        t.monthly.profit < 1000.0,
        "goal should not be met, profit = {}",
        t.monthly.profit
    );
    assert!(t.monthly.profit < state.monthly_goal(0) as f64);
}

#[test]
fn yearly_ref_check_matches_sum_of_monthly_goals() {
    // The Yearly ref block compares sum of 12 monthly goals vs target yearly.
    // When the sum meets the target, the check should pass.
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    // Default min_monthly_net_profit = 500 -> 12 * 500 = 6000.
    state.settings.target_yearly_net_profit = 6000;
    let year_sum: i64 = (0..12).map(|m| state.monthly_goal(m) + state.fix_costs(m)).sum();
    assert_eq!(year_sum, 6000);
    assert!(year_sum >= state.settings.target_yearly_net_profit);

    // Lower the target -> check passes (sum >= target).
    // Raise the target above 6000 -> check fails.
    state.settings.target_yearly_net_profit = 7000;
    let year_sum: i64 = (0..12).map(|m| state.monthly_goal(m) + state.fix_costs(m)).sum();
    assert!(year_sum < state.settings.target_yearly_net_profit);
}

#[test]
fn yearly_ref_sum_includes_monthly_fix_costs() {
    // The 12x-mo sum must include each month's fix costs, not just the raw
    // net-profit goal, since fix costs are part of the burden the products
    // have to cover.
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    // Default min_monthly_net_profit = 500 -> 12 * 500 = 6000 with no fix costs.
    state.settings.target_yearly_net_profit = 6500;
    let year_sum: i64 = (0..12).map(|m| state.monthly_goal(m) + state.fix_costs(m)).sum();
    assert_eq!(year_sum, 6000);
    assert!(year_sum < state.settings.target_yearly_net_profit);

    // Add 500 of fix costs to a single month -> sum rises to 6500 and now
    // meets the target.
    state.month_overrides.fix_costs[0] = 500;
    let year_sum: i64 = (0..12).map(|m| state.monthly_goal(m) + state.fix_costs(m)).sum();
    assert_eq!(year_sum, 6500);
    assert!(year_sum >= state.settings.target_yearly_net_profit);
}

// ---------------------------------------------------------------------------
// Export file numeric content correctness
// ---------------------------------------------------------------------------

#[test]
fn export_results_contains_correct_numeric_values() {
    let dir = std::env::temp_dir().join("tui_export_values_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // net profit = 4.5 - 1.15 = 3.35; duration = 5 min.
    let r = ProductResult {
        name: "Coffee".into(),
        price: 4.5,
        currency: Currency::new("USD"),
        total_cost: 1.15,
        net_profit: 3.35,
        profit_percent: 74.44,
        duration_minutes: 5.0,
    };
    let f = dir.join("p0.txt");
    std::fs::write(&f, "+ stub\n").unwrap();
    let paths = vec![(f, r)];

    let n = 1;
    let monthly_pct = vec![[PCT_TOTAL; 12]];
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
    // Default goal 500. sales = ceil(500/3.35) = ceil(149.25) = 150.
    // minutes = 150 * 5 = 750. annual_sales = 150 * 12 = 1800.

    let status = export_results(&state, &Lang::En);
    assert!(status.contains("exported"), "status was: {}", status);

    let product_file = std::fs::read_to_string(dir.join("p0.simulation_results.txt")).unwrap();
    assert!(product_file.contains("Coffee"), "missing product name");
    assert!(product_file.contains("4.50"), "missing sale price");
    assert!(product_file.contains("1.15"), "missing total cost");
    assert!(product_file.contains("3.35"), "missing net profit per unit");
    assert!(product_file.contains("74.44"), "missing profit margin");
    // Monthly sales 150 in each month row.
    assert!(product_file.contains("150"), "missing monthly sales 150");
    // Annual sales = 1800.
    assert!(product_file.contains("1800"), "missing annual sales 1800");

    let totals_file = std::fs::read_to_string(dir.join("totals.simulation_results.txt")).unwrap();
    assert!(totals_file.contains("1800"), "missing total annual sales 1800");

    // The state file was saved.
    assert!(dir.join("simulation_state.txt").exists(), "state file not saved");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// redistribute_month edge case: setting a product to 0
// ---------------------------------------------------------------------------

#[test]
fn redistribute_month_setting_zero_distributes_to_others() {
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
        prod("C", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.monthly_pct[0][0] = 400;
    state.monthly_pct[1][0] = 300;
    state.monthly_pct[2][0] = 300;
    redistribute_month(&mut state, 0, 0, 0);
    assert_eq!(state.monthly_pct[0][0], 0);
    let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, PCT_TOTAL);
    // B and C split the 1000 equally (500/500).
    assert_eq!(state.monthly_pct[1][0], 500);
    assert_eq!(state.monthly_pct[2][0], 500);
}

// ---------------------------------------------------------------------------
// word_wrap
// ---------------------------------------------------------------------------

#[test]
fn word_wrap_breaks_at_spaces() {
    let lines = word_wrap("hello world foo bar", 10);
    assert_eq!(lines, vec!["hello", "world foo", "bar"]);
}

#[test]
fn word_wrap_hard_splits_long_words() {
    let lines = word_wrap("abcdefghij", 4);
    assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
}

#[test]
fn word_wrap_empty_returns_empty_line() {
    let lines = word_wrap("", 10);
    assert_eq!(lines, vec![""]);
}

// ---------------------------------------------------------------------------
// Fix costs (per-month fixed cost slider)
// ---------------------------------------------------------------------------

#[test]
fn fix_costs_increase_required_sales_and_reduce_profit() {
    // With sufficient capacity, adding fix costs raises the required sales
    // (target = goal + fix) but the achieved net profit stays at the goal
    // (profit = sales*net - fix = goal).
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    state.period = Period::Month(0);

    // No fix costs: required sales = ceil(1000/5) = 200, profit = 1000.
    let s0 = month_shares(&state, 0);
    assert_eq!(s0[0].monthly_sales, 200);
    let mt0 = state.month_totals_for(0);
    assert!((mt0.profit - 1000.0).abs() < 1e-9);

    // Add fix costs 500: target = 1500, required sales = ceil(1500/5) = 300,
    // profit = 300*5 - 500 = 1000 (goal still met).
    state.month_overrides.fix_costs[0] = 500;
    let s1 = month_shares(&state, 0);
    assert_eq!(s1[0].monthly_sales, 300);
    let mt1 = state.month_totals_for(0);
    assert!((mt1.profit - 1000.0).abs() < 1e-9, "profit was {}", mt1.profit);
}

#[test]
fn fix_costs_can_make_goal_unmet_under_capacity() {
    // Under capacity capping, fix costs reduce the achieved net profit below
    // the goal (and can even make it negative).
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    state.settings.min_workday_hours = 1;
    set_all_months(&mut state, 1, 1, 1000);
    state.period = Period::Month(0);
    // capacity = 1*22*60 = 1320 min.
    // Without fix: sales = ceil(1000/5) = 200, minutes = 12000 -> capped 22,
    // profit = 22*5 = 110.
    let mt0 = state.month_totals_for(0);
    assert!((mt0.profit - 110.0).abs() < 1e-9, "profit was {}", mt0.profit);

    // Add fix costs 500: target = 1500, sales = ceil(1500/5) = 300,
    // minutes = 18000, scale = 1320/18000, capped = floor(300*0.0733..) = 22,
    // profit = 22*5 - 500 = -390 < goal 1000.
    state.month_overrides.fix_costs[0] = 500;
    let mt1 = state.month_totals_for(0);
    assert!(mt1.profit < 1000.0, "goal should not be met, profit = {}", mt1.profit);
    assert!(mt1.profit < 0.0, "profit should be negative, got {}", mt1.profit);
}

#[test]
fn fix_costs_persist_in_state_roundtrip() {
    let dir = std::env::temp_dir().join("tui_state_fix_costs_roundtrip");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let f = dir.join("p0.txt");
    std::fs::write(&f, "+ stub\n").unwrap();
    let paths = vec![(f, products.into_iter().next().unwrap())];
    let mut state = make_state(vec![prod("A", 10.0, 5.0, 5.0)]);
    state.folder = dir.clone();
    state.products = paths.clone();
    state.month_overrides.fix_costs[3] = 750;
    save_state(&state);

    let loaded = load_state(&dir, &paths).expect("state should load");
    assert_eq!(loaded.month_overrides.fix_costs[3], 750);

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Graph goal-achievement tick / cross markers
// ---------------------------------------------------------------------------

#[test]
fn graph_renders_tick_for_met_goal_and_cross_for_unmet() {
    use ratatui::backend::TestBackend;
    let products = vec![prod("A", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    set_all_months(&mut state, 8, 1, 1000);
    // Month 0: capacity sufficient -> profit = 1000 = goal -> ✔.
    // Month 1: huge fix costs -> capacity-capped profit < goal -> ✖.
    state.month_overrides.fix_costs[1] = 10000;
    state.tab = Tab::Graph;
    state.period = Period::FullYear;
    state.rebuild_sliders();
    update_parallel_range(&mut state);

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    let has_tick = (0..buf.area.height).any(|y| {
        (0..buf.area.width).any(|x| buf[(x, y)].symbol() == "\u{2714}")
    });
    let has_cross = (0..buf.area.height).any(|y| {
        (0..buf.area.width).any(|x| buf[(x, y)].symbol() == "\u{2716}")
    });
    assert!(has_tick, "no green tick rendered for a met-goal month");
    assert!(has_cross, "no red cross rendered for an unmet-goal month");
}

// ---------------------------------------------------------------------------
// Percentage sliders step by 0.1% (stored in tenths)
// ---------------------------------------------------------------------------

#[test]
fn percent_sliders_step_in_tenths_and_display_one_decimal() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.period = Period::Month(0);
    state.rebuild_sliders();
    let pct_slider = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthPercent(0)))
        .unwrap();
    // Default equal split = 500 (= 50.0%), max is 1000 (= 100.0%), step 1.
    assert_eq!(pct_slider.value, 500);
    assert_eq!(pct_slider.max, PCT_TOTAL);
    assert_eq!(pct_slider.step, 1);
    // The readout shows one decimal place.
    let readout = slider_readout(pct_slider, &state.lang);
    assert!(readout.contains("50.0%"), "readout was: {}", readout);
}

#[test]
fn state_file_is_not_hidden_txt() {
    // The state file name is a non-hidden, plain-text file.
    assert_eq!(STATE_FILE_NAME, "simulation_state.txt");
    assert!(!STATE_FILE_NAME.starts_with('.'));
    assert!(STATE_FILE_NAME.ends_with(".txt"));
}

#[test]
fn collect_txt_files_ignores_state_file() {
    use crate::simulator::collect_txt_files;
    let dir = std::env::temp_dir().join("tui_collect_ignores_state");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("beer.txt"), "+ Beer : 2.7 USD : 0.2 mins\n").unwrap();
    std::fs::write(dir.join("simulation_state.txt"), "# state\n").unwrap();
    std::fs::write(dir.join("beer.simulation_results.txt"), "results\n").unwrap();
    let files = collect_txt_files(&dir);
    let names: Vec<String> = files
        .iter()
        .map(|f| f.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["beer.txt".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}
