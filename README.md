
# Tiny Business Simulator

![preview](https://github.com/DavidValin/tiny-business-simulator/raw/master/preview.gif)

Simulate your next business simply by defining a .txt file for each product in which you define sale prices and costs.

**Example**
```
+ Beer : 2.7 USD : 0.2 mins
  - 0.27 USD labor
  - 0.32 USD beer
  - 0.10 USD cleaning costs
```

### Features

Simulate by the next business goals:

  * the `monthly net profit` goal
  * the `yearly net profit` goal
  * the `parallel products/services` that can be delivered
  * the `workday hours` (the amount of hours a day the business operates)

The simulator targets a **net-profit** goal (income minus costs), not revenue.
Each month, the monthly goal is split across products by their sales-% share,
giving a required sales count per product:

```
required_sales = ceil(share * monthly_goal / net_profit_per_unit)
```

The yearly goal is a **reference target** shown next to the 12 × monthly sum;
the yearly total is the **sum** of the 12 monthly results, so the yearly goal is
only a reference.

`workday hours` and `parallel products` define a monthly production **capacity**
in minutes:

```
capacity_minutes = workday_hours * 22 workdays * 60 * parallel
```

If the required production minutes exceed capacity, sales are scaled down to fit
(the goal cannot be fully met that month). The `parallel products` range is
automatically enforced so the financial goals stay reachable; the slider's own
min/max is recomputed on every change.

The `workdays` figure shown in the results is:

```
workdays = required_hours / (workday_hours * parallel)
```

Obtain simulation results (per product and totals) to meet your business goals:

  * required `monthly sales`
  * required `monthly workdays`
  * required `monthly work time` in minutes and hours
  * required `yearly sales`
  * required `yearly workdays`
  * required `yearly work time` in minutes and hours

Yearly barchart graph with sales number and (sales amount - profit amount) per
month. Above each month's bars a green ✔ marks months whose achieved net profit
meets that month's goal; a red ✖ marks months that fall short.

### Global minimums & monthly overrides

The top-level period tabs select **Full Year** or a single month (Jan..Dec); the
sidebar changes accordingly:

  * **Full Year** — four *global* sliders:
    * `min. workday hours` (default 8)
    * `min. parallel products` (default 1)
    * `min. monthly net profit` (default 500)
    * `target yearly net profit` (default 500, a reference shown next to the
      12 × monthly sum)
  * **A month** — four *override* sliders: `workday hours`,
    `parallel products`, `net profit target` and `fix costs` for that month. Each
    override must be **at least** its global minimum and replaces the minimum
    for that month only. `fix costs` adds a monthly fixed cost (currency units)
    subtracted from the month's net profit, so the required sales grow to cover
    both the goal and the fixed burden.

The monthly net-profit goal used in the `required_sales` formula is therefore
the per-month value (the override when set, otherwise the global `min. monthly
net profit`); the per-month `fix costs` are added to that goal when computing
the required sales, and subtracted from the achieved net profit used by the
goal-achievement check. Workday hours and parallel products are likewise
per-month values floored by their global minimums. The yearly total is the
**sum** of the 12 monthly results, so the `target yearly net profit` slider is
only a reference.

### Sidebar totals

The bottom of the sidebar shows **achievable (capacity-capped)** totals that
always match the chart. The layout depends on the selected period:

  * **Full Year** — left column = `Yearly` totals; right column = `Yearly ref`.
  * **A month** — left column = `Monthly` totals (plus a goal-achievement line);
    right column = `Yearly` totals + `Yearly ref`.

Each period block (`Monthly` / `Yearly`) shows four rows:

  * `sales` — total units sold (after capacity capping).
  * `minutes` — achieved production minutes (capped units × product production
    time).
  * `hours` — `minutes / 60`.
  * `workdays` — `hours / (workday_hours * parallel)`. The annual `workdays`
    sum each month using that month's own `workday hours` / `parallel` override.

For a selected month, an extra line at the end of the `Monthly` block shows
whether the month's net-profit goal was reached: a green ✔ if the month's
achieved net profit ≥ the month's goal, a red ✖ otherwise, followed by the goal
value.

The `Yearly ref` block compares the sum of the 12 monthly net-profit goals
against the `target yearly net profit`:

  * `12x mo` — sum of the 12 monthly net-profit goals (each month's override,
    else the global `min. monthly net profit`).
  * ✔ / ✖ + `goal` — green ✔ if that sum meets the `target yearly net profit`,
    red ✖ otherwise, followed by the target value. The `target yearly net
    profit` is only a reference; the yearly total is the **sum** of the 12
    monthly results.

### Monthly / yearly sales distribution %

Each month's goal is divided between products using percentages. Every month
column always sums to exactly 100%. Percentage sliders move in steps of
**0.10%** (the value is shown with one decimal, e.g. `50.0%`).

  * **A month** (selected via `[`/`]`): each product has a **monthly-%
    slider**. Editing it sets that product's % and redistributes the remainder
    **equally** across all other non-locked products in that month.
  * **Full Year**: each product shows a **yearly-% slider** (the mean of its
    12 monthly values). Editing it propagates the target to every month where
    the product isn't month-locked, redistributing within each month.

The chart draws 12 separate monthly columns, so the mix can vary per month
(seasonal demand — e.g. beer sells more in summer, hot chocolate in winter).
The yearly total is the **sum** of the 12 months.

### Locks

Locks freeze a product's percentage so it is excluded from redistribution.

  * **Yearly lock** (Full Year period, `Space` on a yearly slider): freezes the
    product in **all 12 months**. Month checkboxes render checked and greyed
    out.
  * **Month lock** (a month period, `Space` on a monthly slider): freezes the
    product only for the selected month (disabled if the product is
    yearly-locked).

Locked products keep their fixed share of the 100% pie; the remaining
percentage is split among the unlocked products.

### Exports & state

Pressing `Control+E` exports the simulation to files **and** saves the current
percentages, locks, and settings:

  * one `<product>.simulation_results.txt` per product (stats + 12 monthly rows
    + annual row + workday/parallel)
  * a `totals.simulation_results.txt` aggregating all products
  * a non-hidden, plain-text `simulation_state.txt` file in the product folder
    saving percentages, locks, and settings (editable by hand)

Reopening the app restores the saved distribution.  If products were added or
removed since the save, each month's percentages are re-normalized to sum to 100%.

### Interface language

Set the interface language with `--lang <code>` (default `en`):

  * `en` — English
  * `es` — Spanish
  * `zh` — Chinese
  * `de` — German
  * `ru` — Russian
  * `fr` — French

### Keys

  * `Tab` — move between "Products" and "Graph" sub-tabs
  * `Shift+Tab` (or `o`) — move between sidebar and main area (`o` works on
    all terminals, including raw Linux VTs where Shift+Tab is indistinguishable
    from Tab)
  * `[` / `]` — move the top-level period selection left / right
    (`Full Year`, `Jan`, `Feb`, … `Dec`)
  * `Up`/`Down` — scroll the main area (Products tab) or navigate the sidebar
    sliders
  * `Left`/`Right` — adjust the focused slider
  * `Space` — toggle the lock checkbox of the focused product slider
  * `Control+E` — export the simulation per product and the totals to files
  * `Control+H` — full-screen help on how the simulator works
  * `q` / `Esc` — quit

### Quickstart

Installation:

* via cargo: `cargo install tiny-business-simulator`
* via git source code: `make build && make install`

Run program:

```bash
tiny-business-simulator ./sample_business
```

### Define a business with products/services

See `./sample_business` folder for a sample business with 10 products.

For each product create a .txt file defining:

```
+ <product_name> : <sale_price> <currency> : <production_time> <production_time_units>
  - <cost_a_price> <currency> <cost_a_description>
  - <cost_b_price> <currency> <cost_b_description>
  - <cost_c_price> <currency> <cost_c_description>
```

* first line is the product details
* define as many line costs as you need to produce and deliver such product
* supported currencies: any 3-letter ISO 4217 code (e.g. `USD`, `EUR`, `GBP`, `JPY`, `CAD`, `MXN`, `CNY`)
* supported production-time units: `mins`, `hours`
* products whose net profit (price - total cost) is zero or negative are skipped
* files matching `*.simulation_results.txt` and `simulation_state.txt` are ignored by the loader

Example of a product (`./sample_business/beer.txt`):

```
+ Beer : 2.7 USD : 0.2 mins
  - 0.27 USD labor
  - 0.32 USD beer
  - 0.10 USD cleaning costs
```

#### Run simulation
```
./tiny-business-simulator ./sample_business
```

Run with a specific language:
```
./tiny-business-simulator --lang es ./sample_business
```

Parse and print all product definitions without launching the simulator:
```
./tiny-business-simulator --list ./sample_business
```

#### Install system wide
```
make install
```
