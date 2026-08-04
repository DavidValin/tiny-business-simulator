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
        selected_month: 0,
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
    };
    state.rebuild_sliders();
    state
}

/// Set a settings slider value by kind (workday / parallel / goals).
fn set_setting(state: &mut AppState, kind: SliderKind, value: i64) {
    if let Some(s) = state.sliders.iter_mut().find(|s| s.kind == kind) {
        s.value = value;
    }
}

/// Read a settings slider value by kind.
#[allow(dead_code)]
fn get_setting(state: &AppState, kind: SliderKind) -> i64 {
    state.slider_value(kind)
}

#[test]
fn share_for_month_normalizes_percentages() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let state = make_state(products);
    // Equal 50/50 split -> equal shares in every month.
    for m in 0..12 {
        assert!((state.share_for_month(0, m) - 0.5).abs() < 1e-9);
        assert!((state.share_for_month(1, m) - 0.5).abs() < 1e-9);
    }
}

#[test]
fn share_for_month_all_zero_splits_equally() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    // Zero every month for every product -> equal split fallback.
    for p in &mut state.monthly_pct {
        *p = [0; 12];
    }
    assert!((state.share_for_month(0, 0) - 0.5).abs() < 1e-9);
}

#[test]
fn yearly_pct_is_mean_of_months() {
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    // Make Jan 80/20 and the rest 50/50 -> mean for A = (80 + 11*50)/12.
    state.monthly_pct[0][0] = 80;
    state.monthly_pct[1][0] = 20;
    let expected: f64 = (80.0 + 11.0 * 50.0) / 12.0;
    assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
}

#[test]
fn month_totals_scales_down_when_capacity_exceeds() {
    // One product, net profit 5, duration 60 min.  Goal 1000 -> 200 sales
    // -> 12000 required minutes.  With 1h/day, 1 parallel, 22 days ->
    // capacity = 1*22*60 = 1320 min.  Scale = 1320/12000 = 0.11 ->
    // floor(200*0.11) = 22 units, amount = 22*10 = 220.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
    set_setting(&mut state, SliderKind::WorkdayHours, 1);
    set_setting(&mut state, SliderKind::Parallel, 1);
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
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
    set_setting(&mut state, SliderKind::WorkdayHours, 24);
    set_setting(&mut state, SliderKind::Parallel, 10);
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
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
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
        kind: SliderKind::WorkdayHours,
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
        kind: SliderKind::WorkdayHours,
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
    // 50/30/20. Raise A to 60 -> remainder 40 split EQUALLY across the two
    // non-locked others (including zeros): 20 / 20.
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
    // A=50, B=30, C=0. Raise A to 60 -> remainder 40 split equally across
    // ALL non-locked others (including C at 0): B=20, C=20. (New behaviour:
    // zeros are receivers, unlike the old yearly-only redistribute.)
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
    // A=50, B=30, C=20. Lock B (month-lock) at 30, raise A to 60: remainder
    // 10 goes only to the non-locked C.
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
    assert_eq!(state.monthly_pct[1][0], 30); // frozen
    assert_eq!(state.monthly_pct[2][0], 10); // absorbed the whole remainder
}

#[test]
fn redistribute_month_clamped_by_locked_room() {
    // Lock B=30 and C=50 (locked_sum=80). Raise A above the available 20:
    // it clamps to 20 so the sum stays exactly 100.
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
    // Yearly-lock on B freezes it in every month.
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.yearly_locked[1] = true;
    let before = state.monthly_pct[1][3];
    redistribute_month(&mut state, 3, 0, 60);
    assert_eq!(state.monthly_pct[1][3], before);
    // B is frozen at 50, so A is clamped to 100 - 50 = 50 (not 60).
    assert_eq!(state.monthly_pct[0][3], 50);
    assert_eq!(state.monthly_pct[1][3], 50);
}

// --- edit_yearly (propagation across all 12 months) ---------------------

#[test]
fn edit_yearly_propagates_to_all_months() {
    // 2 products 50/50. Edit yearly A to 80: every month becomes A=80/B=20
    // (no month-locks), and the yearly mean recompute = 80.
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
    // Lock A in month 3 at 50. Edit yearly A to 80: month 3 stays 50, the
    // other 11 months become 80. Yearly mean = (11*80 + 50)/12 ≈ 77.5,
    // NOT 80 — the "unless locked" behaviour.
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.month_locked[0][3] = true;
    edit_yearly(&mut state, 0, 80);
    assert_eq!(state.monthly_pct[0][3], 50); // month-locked, unchanged
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
    // 2 products 50/50. Edit month 0 A to 80 -> month 0 = 80/20, other
    // months stay 50/50. Yearly A = (80 + 11*50)/12 ≈ 52.5 -> 52 (rounded).
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.selected_month = 0;
    redistribute_month(&mut state, 0, 0, 80);
    let expected: f64 = (80.0 + 11.0 * 50.0) / 12.0;
    assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
}

#[test]
fn yearly_lock_renders_month_checkbox_checked_and_greyed() {
    // When a product is yearly-locked, its monthly slider in the Graph tab
    // must render locked (checked) regardless of month_locked.
    let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
    let mut state = make_state(products);
    state.yearly_locked[0] = true;
    state.tab = Tab::Graph;
    state.rebuild_sliders();
    let a_month = state
        .sliders
        .iter()
        .find(|s| matches!(s.kind, SliderKind::MonthPercent(0)))
        .unwrap();
    assert!(a_month.locked, "monthly slider must be locked when yearly-locked");
    // Toggling its month-lock via Space must be ignored (yearly-locked).
    // (Covered by handle_key logic; here we just assert the effective lock.)
}

// --- parallel range / totals / export -----------------------------------

#[test]
fn parallel_slider_range_caps_to_workday_budget() {
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
    set_setting(&mut state, SliderKind::YearlyGoal, 12000);
    set_setting(&mut state, SliderKind::WorkdayHours, 8);
    set_setting(&mut state, SliderKind::Parallel, 1);
    update_parallel_range(&mut state);
    let p = state
        .sliders
        .iter()
        .find(|s| s.kind == SliderKind::Parallel)
        .unwrap();
    assert_eq!(p.min, 1);
    assert_eq!(p.max, 25);
    assert!(p.value >= p.min && p.value <= p.max);
}

#[test]
fn parallel_slider_clamps_when_goal_raises_min() {
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_setting(&mut state, SliderKind::MonthlyGoal, 100000);
    set_setting(&mut state, SliderKind::YearlyGoal, 0);
    set_setting(&mut state, SliderKind::WorkdayHours, 1);
    set_setting(&mut state, SliderKind::Parallel, 1);
    update_parallel_range(&mut state);
    let p = state
        .sliders
        .iter()
        .find(|s| s.kind == SliderKind::Parallel)
        .unwrap();
    assert_eq!(p.min, 667);
    assert_eq!(p.value, 667);
    assert!(p.max >= p.min);
}

#[test]
fn compute_totals_monthly_and_annual() {
    // Two products, equal 50/50 split, monthly goal 1000, all 12 months
    // equal. A: net 5, dur 5 -> 100 sales / 500 min. B: net 10, dur 10 ->
    // 50 sales / 500 min. Monthly totals: 150 sales, 1000 min. Annual =
    // 12 * monthly (all months equal).
    let products = vec![prod("A", 6.0, 1.0, 5.0), prod("B", 12.0, 2.0, 10.0)];
    let mut state = make_state(products);
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
    set_setting(&mut state, SliderKind::YearlyGoal, 12000);
    set_setting(&mut state, SliderKind::WorkdayHours, 8);
    set_setting(&mut state, SliderKind::Parallel, 1);
    let t = compute_totals(&state);
    assert_eq!(t.monthly.sales, 150);
    assert!((t.monthly.minutes - 1000.0).abs() < 1e-6);
    assert_eq!(t.annual.sales, 150 * 12);
    assert!((t.annual.minutes - 12000.0).abs() < 1e-6);
    assert_eq!(t.workday_hours, 8);
    assert_eq!(t.parallel, 1);
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
        selected_month: 0,
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
    };
    state.rebuild_sliders();
    set_setting(&mut state, SliderKind::MonthlyGoal, 500);

    let status = export_results(&state, &Lang::En);
    assert!(status.contains("exported"), "status was: {}", status);

    let product_file = std::fs::read_to_string(dir.join("p0.simulation_results.txt")).unwrap();
    // All 12 month abbreviations appear in the per-product file.
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

    let settings_y = (0..buf.area.height)
        .find_map(|y| find_in_row(&buf, y, "Settings").map(|_| y))
        .expect("Settings title not rendered");
    let header = (settings_y..buf.area.height)
        .find_map(|y| {
            let w = find_in_row(&buf, y, "Workday")?;
            let m = find_in_row(&buf, y, "Monthly")?;
            Some((y, w, m))
        })
        .expect("Workday/Monthly not rendered side by side in Settings");
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
fn graph_sidebar_renders_month_selector() {
    use ratatui::backend::TestBackend;
    let products = vec![
        prod("A", 10.0, 5.0, 5.0),
        prod("B", 10.0, 5.0, 5.0),
    ];
    let mut state = make_state(products);
    state.tab = Tab::Graph;
    state.selected_month = 5; // Jun
    state.rebuild_sliders();
    update_parallel_range(&mut state);

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // The Graph sidebar's top-block title carries the selected month name.
    assert!(
        find_in_row(&buf, 0, "Jun (% sales)").is_some()
            || (0..buf.area.height).any(|y| find_in_row(&buf, y, "Jun").is_some()),
        "selected month Jun not rendered in Graph sidebar title"
    );
    // The month selector slider label "Month" is rendered.
    assert!(
        (0..buf.area.height).any(|y| find_in_row(&buf, y, "Month").is_some()),
        "Month selector not rendered"
    );
}

#[test]
fn save_then_load_state_roundtrip() {
    let dir = std::env::temp_dir().join("tui_state_roundtrip_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Two product definition files.
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
        selected_month: 3,
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
    };
    state.rebuild_sliders();

    // Customize: product 0 = 80%, product 1 = 20% in Jan (month 0).
    state.monthly_pct[0][0] = 80;
    state.monthly_pct[1][0] = 20;
    // Lock product 0 in month 3.
    state.month_locked[0][3] = true;
    // Yearly-lock product 1.
    state.yearly_locked[1] = true;
    // Change a setting.
    for s in state.sliders.iter_mut() {
        if s.kind == SliderKind::MonthlyGoal {
            s.value = 500;
        }
        if s.kind == SliderKind::WorkdayHours {
            s.value = 6;
        }
    }

    // Save state.
    save_state(&state);

    // Load state back.
    let loaded = load_state(&dir, &paths).expect("state should load");
    assert_eq!(loaded.monthly_pct[0][0], 80);
    assert_eq!(loaded.monthly_pct[1][0], 20);
    assert!(loaded.month_locked[0][3]);
    assert!(loaded.yearly_locked[1]);
    assert_eq!(loaded.selected_month, 3);
    assert_eq!(loaded.monthly_goal, 500);
    assert_eq!(loaded.workday_hours, 6);

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

    // Now load with 3 products (p2 was added since the save).
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
    // Create p2.txt so the file exists.
    std::fs::write(dir.join("p2.txt"), "+ stub\n").unwrap();

    let loaded = load_state(&dir, &paths).expect("state should load");
    // p0 and p1 keep their saved percentages.
    assert_eq!(loaded.monthly_pct[0][0], 80);
    assert_eq!(loaded.monthly_pct[1][0], 20);
    // p2 was not in the save → 0. After normalization the month sums to
    // 100 (80+20+0=100, already correct).
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

    // Save state with 3 products (p0=34, p1=33, p2=33 in Jan → sum=100).
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

    // Load with 2 products (p2 was removed since the save).
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
    // After normalization the month must sum to 100 even though p2 is gone
    // (34+33=67 → scaled to 100).
    let sum: i64 = loaded.monthly_pct.iter().map(|p| p[0]).sum();
    assert_eq!(sum, 100, "month must sum to 100 after product removal");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_state_normalizes_when_all_products_locked() {
    let dir = std::env::temp_dir().join("tui_state_all_locked_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Save state with 3 products, ALL yearly-locked, with values that cause
    // rounding drift: 34 + 34 + 34 = 102. After scaling (each becomes
    // round(34/102*100)=33 → sum 99, diff=+1) the drift fix must apply even
    // though no non-locked product exists.
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
    // When capacity is exceeded, the Totals sidebar must show the same
    // achievable (capped) sales as the chart, not the required (uncapped)
    // figures.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
    set_setting(&mut state, SliderKind::WorkdayHours, 1);
    set_setting(&mut state, SliderKind::Parallel, 1);
    // Chart: capacity = 1*22*60 = 1320 min, required = 200*60 = 12000 min.
    // Scale = 1320/12000 = 0.11 -> floor(200*0.11) = 22 units.
    let mt = state.month_totals_for(state.selected_month);
    assert_eq!(mt.units, 22);
    // Totals sidebar must agree with the chart.
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
    // With capacity exceeded, the donut should show < 100% of the yearly goal.
    let products = vec![prod("A", 10.0, 5.0, 60.0)];
    let mut state = make_state(products);
    set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
    set_setting(&mut state, SliderKind::YearlyGoal, 12000);
    set_setting(&mut state, SliderKind::WorkdayHours, 1);
    set_setting(&mut state, SliderKind::Parallel, 1);

    // Capped annual sales = 22/month * 12 = 264 (not 200*12=2400).
    let capped_annual: i64 = (0..12).map(|m| state.capped_product_sales(m)[0]).sum();
    assert_eq!(capped_annual, 264);

    // The yearly profit from capped sales is well below the yearly goal.
    let yearly_profit = capped_annual as f64 * 5.0; // 1320
    let of_goal = yearly_profit / 12000.0 * 100.0; // 11%
    assert!(
        of_goal < 100.0,
        "donut should show <100% when capacity is exceeded, got {:.1}%",
        of_goal
    );

    // Uncapped would show 200% — verify the capped value differs.
    let uncapped_annual: i64 = (0..12)
        .map(|m| month_shares(&state, m)[0].monthly_sales)
        .sum();
    assert_ne!(capped_annual, uncapped_annual);
}
