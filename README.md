
## Tiny Business Simulator

Simulate your next business simply by defining a .txt file for each product in which you define sale prices and costs. 

**Example**
```
+ Beer : 2.7 USD : 0.2 min
  - 0.27 USD labor
  - 0.32 USD beer
  - 0.10 USD cleaning costs
```

![preview](https://github.com/DavidValin/tiny-business-simulator/raw/master/preview.png)

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

To export the data to files: press `Control+E` (it exports the simulation per product and the totals).
To move between sidebar and main area: press `Tab`
To move between "Products" and "Graph" tabs: press `Shift+Tab`

#### Quickstart

```
make
./tiny-business-simulator sample_business
```

#### Define a business with products/services

See `./sample_business` folder for a sample business with 10 products.

For each product create a .txt file defining:

```
+ <product_name> : <sale_price> USD : <production_time> <production_time_units> 
  - <cost_a_price> USD <cost_a_description>
  - <cost_b_price> USD <cost_b_description>
  - <cost_c_price> USD <cost_c_description>
```

* first line is the product details
* define as many line costs as you need to produce and deliver such product

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

#### Install system wide
```
make install
```

