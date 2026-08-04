
## Tiny Business Simulator

Simulate your next business simply by defining a .txt file for each product in which you define sale prices and costs.

**Example**
```
+ Beer : 2.7 USD : 0.2 mins
  - 0.27 USD labor
  - 0.32 USD beer
  - 0.10 USD cleaning costs
```

![preview](https://github.com/DavidValin/tiny-business-simulator/raw/master/preview.gif)

#### Features

Simulate by the next business goals:

  * `monthly net profit`
  * `yearly net profit`
  * `parallel products/services` that can be delivered
  * `workweek hours`

Obtain simulation results (per product and totals) to meet your business goals:

  * required `monthly sales`
  * required `monthly workdays`
  * required `monthly work time` in minutes and hours
  * required `yearly sales`
  * required `yearly workdays`
  * required `yearly work time` in minutes and hours

Yearly barchart graph with sales number and (sales amount - profit amount) per month

It automatically enforces 'parallel products' range to meet the financial goals.

##### Per-month sales distribution

Customize the percentage of sales per product for each month of the year, so the
simulation reflects seasonal demand (e.g. beer sells more in summer, hot
chocolate in winter).  The distribution is controlled from the **Graph** tab:

  * A **Month selector** at the top of the sidebar lets you pick which month to
    edit (January by default).
  * Below it, every product shows a **percentage slider** and a **lock
    checkbox**.  The remaining percentage is redistributed equally across all
    non-locked products in that month.
  * The **Products** tab shows a **yearly percentage** per product (the mean of
    the 12 monthly values).  Editing the yearly percentage propagates the
    target to every month where the product isn't month-locked.

Two levels of locking:

  * **Yearly lock** (Products tab): freezes the product's percentage in all 12
    months (the month checkboxes render checked and greyed out).
  * **Month lock** (Graph tab): freezes the product's percentage only for the
    selected month.

The chart renders 12 distinct monthly columns (each month's mix may differ) and
the yearly total is the **sum** of the 12 months.

##### State persistence

Pressing `Control+E` exports the simulation to files **and** saves the current
percentages, locks, and settings to a hidden `.simulation_state` file in the
product folder.  Reopening the app restores the saved distribution.  If
products were added or removed since the save, each month's percentages are
re-normalized to sum to 100.

##### Interface language

Set the interface language with `--lang <code>` (default `en`):

  * `en` — English
  * `es` — Spanish
  * `zh` — Chinese
  * `de` — German
  * `ru` — Russian
  * `fr` — French

#### Keys

  * `Tab` — move between sidebar and main area
  * `Shift+Tab` — move between "Products" and "Graph" tabs
  * `Up`/`Down` — scroll the main area (Products tab) or navigate the sidebar
    sliders
  * `Left`/`Right` — adjust the focused slider (or change the selected month
    on the Graph tab's month selector)
  * `Space` — toggle the lock checkbox of the focused product slider
  * `Control+E` — export the simulation per product and the totals to files
  * `q` / `Esc` — quit

#### Quickstart

```
make
./tiny-business-simulator sample_business
```

#### Define a business with products/services

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
