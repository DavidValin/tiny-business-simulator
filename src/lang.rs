// Internationalization dictionary.
//
// Every user-facing string lives here as a template containing `{0}`..`{5}`
// positional placeholders. Templates are filled at runtime through
// [`fmt`], because Rust's `format!` macro requires a string literal and so
// cannot be used with a dictionary lookup.
//
// Numbers are pre-formatted (e.g. with `{:.2}`) by the caller and passed in as
// already-rendered strings, so the templates only ever use plain `{n}`.
//
// Six languages are provided: en (default), es, zh, de, ru, fr.

// ---------------------------------------------------------------------------
// Dictionary
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct Dict {
    // --- cli.rs ---------------------------------------------------------------
    pub usage_label: &'static str,
    pub usage_interactive: &'static str,
    pub usage_list: &'static str,
    pub usage_lang: &'static str,
    pub no_txt_files: &'static str,
    pub cannot_read_file: &'static str,
    pub file_colon_msg: &'static str,
    pub parsed_products_header: &'static str,
    pub no_valid_products: &'static str,
    pub file_label: &'static str,
    pub not_a_directory: &'static str,
    pub unknown_lang: &'static str,

    // --- parser.rs : Display + errors ----------------------------------------
    pub cost_display: &'static str,
    pub product_label: &'static str,
    pub sale_price_label: &'static str,
    pub production_time_label: &'static str,
    pub costs_header: &'static str,
    pub no_costs: &'static str,
    pub time_mins: &'static str,
    pub time_hours: &'static str,
    pub err_duplicate_product_line: &'static str,
    pub err_line_prefix: &'static str,
    pub err_unexpected_line: &'static str,
    pub err_missing_product_line: &'static str,
    pub err_product_sections: &'static str,
    pub err_product_plus: &'static str,
    pub err_empty_product_name: &'static str,
    pub err_sale_section: &'static str,
    pub err_invalid_sale_price: &'static str,
    pub err_invalid_sale_currency: &'static str,
    pub err_time_section: &'static str,
    pub err_invalid_prod_time: &'static str,
    pub err_invalid_time_unit: &'static str,
    pub err_cost_minus: &'static str,
    pub err_cost_tokens: &'static str,
    pub err_invalid_cost_price: &'static str,
    pub err_invalid_cost_currency: &'static str,

    // --- simulator.rs : result file + stats ----------------------------------
    pub result_product: &'static str,
    pub result_sale_price: &'static str,
    pub result_total_cost: &'static str,
    pub result_net_profit_unit: &'static str,
    pub result_profit_margin: &'static str,
    pub result_prod_time: &'static str,
    pub result_monthly_goal: &'static str,
    pub result_monthly_time: &'static str,
    pub result_month_row: &'static str,
    pub result_annual_goal: &'static str,
    pub result_annual_time: &'static str,
    pub result_workday: &'static str,
    pub result_parallel: &'static str,
    pub required_sales_needle: &'static str,

    // --- simulator.rs : interactive prompts + warnings -----------------------
    pub warn_net_profit_nonpositive: &'static str,
    pub prompt_monthly_goal: &'static str,
    pub prompt_annual_goal: &'static str,
    pub prompt_monthly_goal_plain: &'static str,
    pub prompt_annual_goal_plain: &'static str,
    pub err_read_monthly_goal: &'static str,
    pub err_read_annual_goal: &'static str,
    pub prompt_workday_hours: &'static str,
    pub validate_workday_range: &'static str,
    pub err_read_workday: &'static str,
    pub prompt_parallel_products: &'static str,
    pub validate_parallel_range: &'static str,
    pub err_read_parallel: &'static str,
    pub sales_needed_header: &'static str,
    pub monthly_label: &'static str,
    pub annual_label: &'static str,
    pub err_write_result_file: &'static str,

    // --- simulator.rs : multi-product split ----------------------------------
    pub warn_product_excluded: &'static str,
    pub warn_no_positive_products: &'static str,
    pub random_split_header: &'static str,
    pub split_item: &'static str,
    pub per_product_header: &'static str,
    pub per_product_monthly: &'static str,
    pub per_product_annual: &'static str,
    pub totals_header: &'static str,
    pub total_monthly_sales: &'static str,
    pub total_month_row: &'static str,
    pub total_annual_sales: &'static str,
    pub err_write_result_file_for: &'static str,
    pub err_write_totals_file: &'static str,

    // --- simulator.rs : menu + file selection --------------------------------
    pub warn_no_txt_files: &'static str,
    pub menu_prompt: &'static str,
    pub err_menu: &'static str,
    pub warn_no_selection: &'static str,
    pub err_read_file: &'static str,

    // --- simulator.rs : one-line result summary ------------------------------
    pub summary_net_profit: &'static str,
    pub summary_margin: &'static str,
    pub summary_monthly: &'static str,
    pub summary_annual: &'static str,

    // --- tui.rs : tabs, sidebar titles, slider labels ------------------------
    pub tui_tab_products: &'static str,
    pub tui_tab_graph: &'static str,
    pub tui_sidebar_month: &'static str,
    pub tui_sidebar_settings: &'static str,
    pub tui_sidebar_totals: &'static str,
    pub tui_products_yearly: &'static str,
    pub tui_month_pct_sales: &'static str,
    pub tui_slider_workday: &'static str,
    pub tui_slider_parallel: &'static str,
    pub tui_slider_monthly_goal: &'static str,
    pub tui_slider_yearly_goal: &'static str,
    pub tui_slider_month: &'static str,
    pub tui_parallel_label: &'static str,
    pub tui_lock_year: &'static str,
    pub tui_lock_month: &'static str,

    // --- tui.rs : chart legend / stats ---------------------------------------
    pub tui_yearly_sales: &'static str,
    pub tui_legend_units: &'static str,
    pub tui_legend_profit: &'static str,
    pub tui_legend_cost: &'static str,
    pub tui_axis_max: &'static str,
    pub tui_max: &'static str,
    pub tui_profit: &'static str,
    pub tui_yearly: &'static str,

    // --- tui.rs : donut captions ---------------------------------------------
    pub tui_donut_margin: &'static str,
    pub tui_donut_vs_year: &'static str,

    // --- tui.rs : totals column labels ---------------------------------------
    pub tui_label_monthly: &'static str,
    pub tui_label_yearly: &'static str,
    pub tui_label_settings: &'static str,
    pub tui_label_sales: &'static str,
    pub tui_label_min: &'static str,
    pub tui_label_hours: &'static str,
    pub tui_label_workdays: &'static str,
    pub tui_label_workday: &'static str,
    pub tui_label_parallel: &'static str,
    pub tui_label_yearly_ref: &'static str,
    pub tui_label_12x_mo: &'static str,
    pub tui_label_goal: &'static str,
    pub tui_suffix_hours: &'static str,

    // --- tui.rs : footer / status / regions ----------------------------------
    pub tui_region_main: &'static str,
    pub tui_region_sidebar: &'static str,
    pub tui_footer: &'static str,
    pub tui_footer_status: &'static str,
    pub tui_export_error: &'static str,
    pub tui_export_error_totals: &'static str,
    pub tui_exported: &'static str,
    pub tui_no_products: &'static str,

    // --- tui.rs : help screen (Ctrl+H) ---------------------------------------
    pub tui_help_title: &'static str,
    pub tui_help_text: &'static str,
}

macro_rules! define_dict {
    (
        $(
            $field:ident : [$en:expr, $es:expr, $zh:expr, $de:expr, $ru:expr, $fr:expr $(,)?]
        ),* $(,)?
    ) => {
        pub static EN: Dict = Dict { $($field: $en,)* };
        pub static ES: Dict = Dict { $($field: $es,)* };
        pub static ZH: Dict = Dict { $($field: $zh,)* };
        pub static DE: Dict = Dict { $($field: $de,)* };
        pub static RU: Dict = Dict { $($field: $ru,)* };
        pub static FR: Dict = Dict { $($field: $fr,)* };
    };
}

define_dict! {
    // --- cli.rs ---------------------------------------------------------------
    usage_label: ["Usage:", "Uso:", "用法：", "Verwendung:", "Использование:", "Utilisation :"],
    usage_interactive: [
        "  {0} <root_folder>          Launch the interactive product simulator menu",
        "  {0} <carpeta_raíz>          Inicia el menú interactivo del simulador de productos",
        "  {0} <根目录>          启动交互式产品模拟器菜单",
        "  {0} <Wurzelordner>          Startet das interaktive Produkt-Simulator-Menü",
        "  {0} <корневая_папка>          Запустить интерактивное меню симулятора продуктов",
        "  {0} <dossier_racine>          Lance le menu interactif du simulateur de produits",
    ],
    usage_list: [
        "  {0} --list <root_folder>   Parse and print all product definitions",
        "  {0} --list <carpeta_raíz>   Analiza e imprime todas las definiciones de productos",
        "  {0} --list <根目录>   解析并打印所有产品定义",
        "  {0} --list <Wurzelordner>   Analysiert und gibt alle Produktdefinitionen aus",
        "  {0} --list <корневая_папка>   Разобрать и вывести все определения продуктов",
        "  {0} --list <dossier_racine>   Analyser et afficher toutes les définitions de produits",
    ],
    usage_lang: [
        "  {0} --lang <lang_code> <root_folder>   Set interface language (en, es, zh, de, ru, fr)",
        "  {0} --lang <código_idioma> <carpeta_raíz>   Selecciona el idioma (en, es, zh, de, ru, fr)",
        "  {0} --lang <语言代码> <根目录>   设置界面语言 (en, es, zh, de, ru, fr)",
        "  {0} --lang <sprachcode> <Wurzelordner>   Schnittstellensprache wählen (en, es, zh, de, ru, fr)",
        "  {0} --lang <код_языка> <корневая_папка>   Задать язык интерфейса (en, es, zh, de, ru, fr)",
        "  {0} --lang <code_langue> <dossier_racine>   Définir la langue (en, es, zh, de, ru, fr)",
    ],
    no_txt_files: [
        "No .txt files found in '{0}'",
        "No se encontraron archivos .txt en '{0}'",
        "在 '{0}' 中未找到 .txt 文件",
        "Keine .txt-Dateien in '{0}' gefunden",
        "В '{0}' не найдено .txt-файлов",
        "Aucun fichier .txt trouvé dans « {0} »",
    ],
    cannot_read_file: [
        "{0}: cannot read file: {1}",
        "{0}: no se puede leer el archivo: {1}",
        "{0}: 无法读取文件: {1}",
        "{0}: Datei konnte nicht gelesen werden: {1}",
        "{0}: не удалось прочитать файл: {1}",
        "{0} : impossible de lire le fichier : {1}",
    ],
    file_colon_msg: [
        "{0}: {1}",
        "{0}: {1}",
        "{0}: {1}",
        "{0}: {1}",
        "{0}: {1}",
        "{0} : {1}",
    ],
    parsed_products_header: [
        "=== Parsed products ({0} valid) ===",
        "=== Productos analizados ({0} válidos) ===",
        "=== 已解析的产品（{0} 个有效）===",
        "=== Analysierte Produkte ({0} gültig) ===",
        "=== Разобранные продукты ({0} действительных) ===",
        "=== Produits analysés ({0} valides) ===",
    ],
    no_valid_products: [
        "(no valid products found)",
        "(no se encontraron productos válidos)",
        "(未找到有效产品)",
        "(keine gültigen Produkte gefunden)",
        "(действительных продуктов не найдено)",
        "(aucun produit valide trouvé)",
    ],
    file_label: [
        "File: {0}",
        "Archivo: {0}",
        "文件: {0}",
        "Datei: {0}",
        "Файл: {0}",
        "Fichier : {0}",
    ],
    not_a_directory: [
        "Error: '{0}' is not a directory",
        "Error: '{0}' no es un directorio",
        "错误：'{0}' 不是目录",
        "Fehler: '{0}' ist kein Verzeichnis",
        "Ошибка: '{0}' не является каталогом",
        "Erreur : « {0} » n'est pas un répertoire",
    ],
    unknown_lang: [
        "Error: unknown language code '{0}' (supported: en, es, zh, de, ru, fr)",
        "Error: código de idioma desconocido '{0}' (soportados: en, es, zh, de, ru, fr)",
        "错误：未知的语言代码 '{0}'（支持：en, es, zh, de, ru, fr）",
        "Fehler: unbekannter Sprachcode '{0}' (unterstützt: en, es, zh, de, ru, fr)",
        "Ошибка: неизвестный код языка '{0}' (поддерживаются: en, es, zh, de, ru, fr)",
        "Erreur : code de langue inconnu « {0} » (supportés : en, es, zh, de, ru, fr)",
    ],

    // --- parser.rs : Display + errors ----------------------------------------
    cost_display: [
        "- {0} {1} {2}",
        "- {0} {1} {2}",
        "- {0} {1} {2}",
        "- {0} {1} {2}",
        "- {0} {1} {2}",
        "- {0} {1} {2}",
    ],
    product_label: [
        "Product: {0}",
        "Producto: {0}",
        "产品: {0}",
        "Produkt: {0}",
        "Продукт: {0}",
        "Produit : {0}",
    ],
    sale_price_label: [
        "  Sale price:     {0} {1}",
        "  Precio de venta:     {0} {1}",
        "  售价:     {0} {1}",
        "  Verkaufspreis:     {0} {1}",
        "  Цена продажи:     {0} {1}",
        "  Prix de vente :     {0} {1}",
    ],
    production_time_label: [
        "  Production time: {0} {1}",
        "  Tiempo de producción: {0} {1}",
        "  生产时间: {0} {1}",
        "  Produktionszeit: {0} {1}",
        "  Время производства: {0} {1}",
        "  Temps de production : {0} {1}",
    ],
    costs_header: [
        "  Costs ({0}):",
        "  Costes ({0}):",
        "  成本 ({0}):",
        "  Kosten ({0}):",
        "  Затраты ({0}):",
        "  Coûts ({0}) :",
    ],
    no_costs: [
        "    (none)",
        "    (ninguno)",
        "    (无)",
        "    (keine)",
        "    (нет)",
        "    (aucun)",
    ],
    time_mins: [
        "mins",
        "minutos",
        "分钟",
        "Minuten",
        "мин",
        "min",
    ],
    time_hours: [
        "hours",
        "horas",
        "小时",
        "Stunden",
        "часы",
        "heures",
    ],
    err_duplicate_product_line: [
        "line {0}: duplicate product definition line",
        "línea {0}: definición de producto duplicada",
        "第 {0} 行：重复的产品定义行",
        "Zeile {0}: doppelte Produktdefinitionszeile",
        "строка {0}: повторная строка определения продукта",
        "ligne {0} : ligne de définition de produit en double",
    ],
    err_line_prefix: [
        "line {0}: {1}",
        "línea {0}: {1}",
        "第 {0} 行：{1}",
        "Zeile {0}: {1}",
        "строка {0}: {1}",
        "ligne {0} : {1}",
    ],
    err_unexpected_line: [
        "line {0}: unexpected line, every line must start with '+' or '-'",
        "línea {0}: línea inesperada, cada línea debe empezar con '+' o '-'",
        "第 {0} 行：意外的行，每行必须以 '+' 或 '-' 开头",
        "Zeile {0}: unerwartete Zeile, jede Zeile muss mit '+' oder '-' beginnen",
        "строка {0}: неожиданная строка, каждая строка должна начинаться с '+' или '-'",
        "ligne {0} : ligne inattendue, chaque ligne doit commencer par « + » ou « - »",
    ],
    err_missing_product_line: [
        "missing product definition line (a line starting with '+')",
        "falta la línea de definición de producto (una línea que empieza con '+')",
        "缺少产品定义行（以 '+' 开头的行）",
        "Produktdefinitionszeile fehlt (eine mit '+' beginnende Zeile)",
        "отсутствует строка определения продукта (строка, начинающаяся с '+')",
        "ligne de définition de produit manquante (une ligne commençant par « + »)",
    ],
    err_product_sections: [
        "product line must contain exactly 3 colon-separated sections, found {0}",
        "la línea de producto debe contener exactamente 3 secciones separadas por dos puntos, se encontraron {0}",
        "产品行必须包含恰好 3 个以冒号分隔的部分，找到 {0} 个",
        "Produktzeile muss genau 3 durch Doppelpunkt getrennte Abschnitte enthalten, gefunden {0}",
        "строка продукта должна содержать ровно 3 секции, разделённых двоеточием, найдено {0}",
        "la ligne produit doit contenir exactement 3 sections séparées par deux-points, {0} trouvée(s)",
    ],
    err_product_plus: [
        "product line must start with '+'",
        "la línea de producto debe empezar con '+'",
        "产品行必须以 '+' 开头",
        "Produktzeile muss mit '+' beginnen",
        "строка продукта должна начинаться с '+'",
        "la ligne produit doit commencer par « + »",
    ],
    err_empty_product_name: [
        "product name is empty",
        "el nombre del producto está vacío",
        "产品名称为空",
        "Produktname ist leer",
        "имя продукта пусто",
        "le nom du produit est vide",
    ],
    err_sale_section: [
        "sale section must be '<price> <currency>', found {0} token(s)",
        "la sección de venta debe ser '<precio> <moneda>', se encontraron {0} token(s)",
        "销售部分必须是 '<价格> <货币>'，找到 {0} 个标记",
        "Verkaufsabschnitt muss '<Preis> <Währung>' sein, {0} Token gefunden",
        "секция продажи должна быть '<цена> <валюта>', найдено {0} токен(ов)",
        "la section vente doit être « <prix> <monnaie> », {0} jeton(s) trouvé(s)",
    ],
    err_invalid_sale_price: [
        "invalid sale price '{0}'",
        "precio de venta inválido '{0}'",
        "无效的售价 '{0}'",
        "ungültiger Verkaufspreis '{0}'",
        "недопустимая цена продажи '{0}'",
        "prix de vente invalide « {0} »",
    ],
    err_invalid_sale_currency: [
        "invalid sale currency '{0}': must be a 3-letter ISO 4217 code (e.g. USD, EUR, GBP, JPY)",
        "moneda de venta inválida '{0}': debe ser un código ISO 4217 de 3 letras (ej. USD, EUR, GBP, JPY)",
        "无效的售出货币 '{0}'：必须是3字母 ISO 4217代码（如 USD、EUR、GBP、JPY）",
        "ungültige Verkaufswährung '{0}': muss ein 3-buchstabiger ISO-4217-Code sein (z.B. USD, EUR, GBP, JPY)",
        "недопустимая валюта продажи '{0}': должна быть 3-буквенным кодом ISO 4217 (напр. USD, EUR, GBP, JPY)",
        "monnaie de vente invalide « {0} » : doit être un code ISO 4217 de 3 lettres (ex. USD, EUR, GBP, JPY)",
    ],
    err_time_section: [
        "production-time section must be '<time> <unit>', found {0} token(s)",
        "la sección de tiempo de producción debe ser '<tiempo> <unidad>', se encontraron {0} token(s)",
        "生产时间部分必须是 '<时间> <单位>'，找到 {0} 个标记",
        "Produktionszeit-Abschnitt muss '<Zeit> <Einheit>' sein, {0} Token gefunden",
        "секция времени производства должна быть '<время> <единица>', найдено {0} токен(ов)",
        "la section temps de production doit être « <temps> <unité> », {0} jeton(s) trouvé(s)",
    ],
    err_invalid_prod_time: [
        "invalid production time '{0}'",
        "tiempo de producción inválido '{0}'",
        "无效的生产时间 '{0}'",
        "ungültige Produktionszeit '{0}'",
        "недопустимое время производства '{0}'",
        "temps de production invalide « {0} »",
    ],
    err_invalid_time_unit: [
        "invalid production time unit '{0}': must be one of mins, hours",
        "unidad de tiempo de producción inválida '{0}': debe ser una de mins, hours",
        "无效的生产时间单位 '{0}'：必须是 mins 或 hours",
        "ungültige Produktionszeit-Einheit '{0}': muss mins oder hours sein",
        "недопустимая единица времени производства '{0}': должна быть mins или hours",
        "unité de temps de production invalide « {0} » : doit être mins ou hours",
    ],
    err_cost_minus: [
        "cost line must start with '-'",
        "la línea de coste debe empezar con '-'",
        "成本行必须以 '-' 开头",
        "Kostenzeile muss mit '-' beginnen",
        "строка затрат должна начинаться с '-'",
        "la ligne de coût doit commencer par « - »",
    ],
    err_cost_tokens: [
        "cost line must be '<price> <currency> <description>', found {0} token(s)",
        "la línea de coste debe ser '<precio> <moneda> <descripción>', se encontraron {0} token(s)",
        "成本行必须是 '<价格> <货币> <描述>'，找到 {0} 个标记",
        "Kostenzeile muss '<Preis> <Währung> <Beschreibung>' sein, {0} Token gefunden",
        "строка затрат должна быть '<цена> <валюта> <описание>', найдено {0} токен(ов)",
        "la ligne de coût doit être « <prix> <monnaie> <description> », {0} jeton(s) trouvé(s)",
    ],
    err_invalid_cost_price: [
        "invalid cost price '{0}'",
        "precio de coste inválido '{0}'",
        "无效的成本价格 '{0}'",
        "ungültiger Kostenpreis '{0}'",
        "недопустимая цена затрат '{0}'",
        "prix de coût invalide « {0} »",
    ],
    err_invalid_cost_currency: [
        "invalid cost currency '{0}': must be a 3-letter ISO 4217 code (e.g. USD, EUR, GBP, JPY)",
        "moneda de coste inválida '{0}': debe ser un código ISO 4217 de 3 letras (ej. USD, EUR, GBP, JPY)",
        "无效的成本货币 '{0}'：必须是3字母 ISO 4217代码（如 USD、EUR、GBP、JPY）",
        "ungültige Kostenwährung '{0}': muss ein 3-buchstabiger ISO-4217-Code sein (z.B. USD, EUR, GBP, JPY)",
        "недопустимая валюта затрат '{0}': должна быть 3-буквенным кодом ISO 4217 (напр. USD, EUR, GBP, JPY)",
        "monnaie de coût invalide « {0} » : doit être un code ISO 4217 de 3 lettres (ex. USD, EUR, GBP, JPY)",
    ],

    // --- simulator.rs : result file + stats ----------------------------------
    result_product: [
        "📦 Product: \t\t\t{0}",
        "📦 Producto: \t\t\t{0}",
        "📦 产品: \t\t\t{0}",
        "📦 Produkt: \t\t\t{0}",
        "📦 Продукт: \t\t\t{0}",
        "📦 Produit : \t\t\t{0}",
    ],
    result_sale_price: [
        "💶 Sale price: \t\t{0} {1}",
        "💶 Precio de venta: \t\t{0} {1}",
        "💶 售价: \t\t{0} {1}",
        "💶 Verkaufspreis: \t\t{0} {1}",
        "💶 Цена продажи: \t\t{0} {1}",
        "💶 Prix de vente : \t\t{0} {1}",
    ],
    result_total_cost: [
        "💸 Total cost: \t\t{0} {1}",
        "💸 Coste total: \t\t{0} {1}",
        "💸 总成本: \t\t{0} {1}",
        "💸 Gesamtkosten: \t\t{0} {1}",
        "💸 Общие затраты: \t\t{0} {1}",
        "💸 Coût total : \t\t{0} {1}",
    ],
    result_net_profit_unit: [
        "📈 Net profit (unit): \t{0} {1}",
        "📈 Beneficio neto (unidad): \t{0} {1}",
        "📈 净利润（单位）: \t{0} {1}",
        "📈 Nettogewinn (Einheit): \t{0} {1}",
        "📈 Чистая прибыль (единица): \t{0} {1}",
        "📈 Profit net (unité) : \t{0} {1}",
    ],
    result_profit_margin: [
        "📊 Profit margin: \t{0}%",
        "📊 Margen de beneficio: \t{0}%",
        "📊 利润率: \t{0}%",
        "📊 Gewinnmarge: \t{0}%",
        "📊 Маржа прибыли: \t{0}%",
        "📊 Marge bénéficiaire : \t{0}%",
    ],
    result_prod_time: [
        "🕐 Production time: \t{0} minutes",
        "🕐 Tiempo de producción: \t{0} minutos",
        "🕐 生产时间: \t{0} 分钟",
        "🕐 Produktionszeit: \t{0} Minuten",
        "🕐 Время производства: \t{0} минут",
        "🕐 Temps de production : \t{0} minutes",
    ],
    result_monthly_goal: [
        "🎯 Monthly goal:\t\t{0} {1} → Required sales: {2}",
        "🎯 Meta mensual:\t\t{0} {1} → Ventas necesarias: {2}",
        "🎯 月度目标:\t\t{0} {1} → 所需销售: {2}",
        "🎯 Monatsziel:\t\t{0} {1} → Erforderliche Verkäufe: {2}",
        "🎯 Месячная цель:\t\t{0} {1} → Требуемые продажи: {2}",
        "🎯 Objectif mensuel :\t\t{0} {1} → Ventes requises : {2}",
    ],
    result_monthly_time: [
        "🕐 Total monthly time:\t{0} minutes ({1} hours) ({2} parallel products in {3} workday hours) ({4} workdays)",
        "🕐 Tiempo mensual total:\t{0} minutos ({1} horas) ({2} productos en paralelo en {3} horas de jornada) ({4} jornadas)",
        "🕐 月度总时间:\t{0} 分钟 ({1} 小时) ({2} 个并行产品，{3} 工作小时) ({4} 工作日)",
        "🕐 Gesamtzeit monatlich:\t{0} Minuten ({1} Stunden) ({2} Parallelprodukte in {3} Arbeitstunden) ({4} Arbeitstage)",
        "🕐 Общее время за месяц:\t{0} минут ({1} часов) ({2} параллельных продукта при {3} часах рабочего дня) ({4} рабочих дней)",
        "🕐 Temps mensuel total :\t{0} minutes ({1} heures) ({2} produits parallèles sur {3} heures de travail) ({4} jours de travail)",
    ],
    result_month_row: [
        "{0} {1} → {2} sales  🕐 {3} min ({4} h)",
        "{0} {1} → {2} ventas  🕐 {3} min ({4} h)",
        "{0} {1} → {2} 次销售  🕐 {3} 分钟 ({4} 小时)",
        "{0} {1} → {2} Verkäufe  🕐 {3} Min ({4} Std.)",
        "{0} {1} → {2} продаж  🕐 {3} мин ({4} ч)",
        "{0} {1} → {2} ventes  🕐 {3} min ({4} h)",
    ],
    result_annual_goal: [
        "🎯 Annual goal:\t\t\t{0} {1} → Required sales: {2}",
        "🎯 Meta anual:\t\t\t{0} {1} → Ventas necesarias: {2}",
        "🎯 年度目标:\t\t\t{0} {1} → 所需销售: {2}",
        "🎯 Jahresziel:\t\t\t{0} {1} → Erforderliche Verkäufe: {2}",
        "🎯 Годовая цель:\t\t\t{0} {1} → Требуемые продажи: {2}",
        "🎯 Objectif annuel :\t\t\t{0} {1} → Ventes requises : {2}",
    ],
    result_annual_time: [
        "🕐 Total annual time:\t\t{0} minutes ({1} hours) ({2} parallel products in {3} workday hours) ({4} workdays)",
        "🕐 Tiempo anual total:\t\t{0} minutos ({1} horas) ({2} productos en paralelo en {3} horas de jornada) ({4} jornadas)",
        "🕐 年度总时间:\t\t{0} 分钟 ({1} 小时) ({2} 个并行产品，{3} 工作小时) ({4} 工作日)",
        "🕐 Gesamtzeit jährlich:\t\t{0} Minuten ({1} Stunden) ({2} Parallelprodukte in {3} Arbeitstunden) ({4} Arbeitstage)",
        "🕐 Общее время за год:\t\t{0} минут ({1} часов) ({2} параллельных продукта при {3} часах рабочего дня) ({4} рабочих дней)",
        "🕐 Temps annuel total :\t\t{0} minutes ({1} heures) ({2} produits parallèles sur {3} heures de travail) ({4} jours de travail)",
    ],
    result_workday: [
        "🕐 Workday:\t\t{0} hours",
        "🕐 Jornada laboral:\t\t{0} horas",
        "🕐 工作日:\t\t{0} 小时",
        "🕐 Arbeitstag:\t\t{0} Stunden",
        "🕐 Рабочий день:\t\t{0} часов",
        "🕐 Journée de travail :\t\t{0} heures",
    ],
    result_parallel: [
        "🧵 Parallel products:\t{0}",
        "🧵 Productos en paralelo:\t{0}",
        "🧵 并行产品:\t{0}",
        "🧵 Parallelprodukte:\t{0}",
        "🧵 Параллельные продукты:\t{0}",
        "🧵 Produits parallèles :\t{0}",
    ],
    required_sales_needle: [
        "Required sales:",
        "Ventas necesarias:",
        "所需销售:",
        "Erforderliche Verkäufe:",
        "Требуемые продажи:",
        "Ventes requises :",
    ],

    // --- simulator.rs : interactive prompts + warnings -----------------------
    warn_net_profit_nonpositive: [
        "⚠️ The net profit per unit is {0} {1}. It is not possible to reach a net profit goal like this.",
        "⚠️ El beneficio neto por unidad es {0} {1}. No es posible alcanzar una meta de beneficios netos así.",
        "⚠️ 单位净利润为 {0} {1}。无法以此达到净利润目标。",
        "⚠️ Der Nettogewinn pro Einheit beträgt {0} {1}. Ein Nettogewinnziel ist so nicht erreichbar.",
        "⚠️ Чистая прибыль на единицу составляет {0} {1}. Достичь цели по чистой прибыли таким образом невозможно.",
        "⚠️ Le profit net par unité est de {0} {1}. Impossible d'atteindre un objectif de profit net ainsi.",
    ],
    prompt_monthly_goal: [
        "🎯 What is your MONTHLY net profit goal (in {0})?",
        "🎯 ¿Cuál es tu meta MENSUAL de beneficio neto (en {0})?",
        "🎯 你的月度净利润目标是多少（单位 {0}）？",
        "🎯 Wie hoch ist dein MONATliches Nettogewinnziel (in {0})?",
        "🎯 Какова ваша МЕСЯЧНАЯ цель по чистой прибыли (в {0})?",
        "🎯 Quel est votre objectif MENSUEL de profit net (en {0}) ?",
    ],
    prompt_annual_goal: [
        "🎯 What is your ANNUAL net profit goal (in {0})?",
        "🎯 ¿Cuál es tu meta ANUAL de beneficio neto (en {0})?",
        "🎯 你的年度净利润目标是多少（单位 {0}）？",
        "🎯 Wie hoch ist dein JAHRES-Nettogewinnziel (in {0})?",
        "🎯 Какова ваша ГОДОВАЯ цель по чистой прибыли (в {0})?",
        "🎯 Quel est votre objectif ANNUEL de profit net (en {0}) ?",
    ],
    prompt_monthly_goal_plain: [
        "🎯 What is your MONTHLY net profit goal?",
        "🎯 ¿Cuál es tu meta MENSUAL de beneficio neto?",
        "🎯 你的月度净利润目标是多少？",
        "🎯 Wie hoch ist dein MONATliches Nettogewinnziel?",
        "🎯 Какова ваша МЕСЯЧНАЯ цель по чистой прибыли?",
        "🎯 Quel est votre objectif MENSUEL de profit net ?",
    ],
    prompt_annual_goal_plain: [
        "🎯 What is your ANNUAL net profit goal?",
        "🎯 ¿Cuál es tu meta ANUAL de beneficio neto?",
        "🎯 你的年度净利润目标是多少？",
        "🎯 Wie hoch ist dein JAHRES-Nettogewinnziel?",
        "🎯 Какова ваша ГОДОВАЯ цель по чистой прибыли?",
        "🎯 Quel est votre objectif ANNUEL de profit net ?",
    ],
    err_read_monthly_goal: [
        "could not read monthly goal: {0}",
        "no se pudo leer la meta mensual: {0}",
        "无法读取月度目标: {0}",
        "Monatsziel konnte nicht gelesen werden: {0}",
        "не удалось прочитать месячную цель: {0}",
        "impossible de lire l'objectif mensuel : {0}",
    ],
    err_read_annual_goal: [
        "could not read annual goal: {0}",
        "no se pudo leer la meta anual: {0}",
        "无法读取年度目标: {0}",
        "Jahresziel konnte nicht gelesen werden: {0}",
        "не удалось прочитать годовую цель: {0}",
        "impossible de lire l'objectif annuel : {0}",
    ],
    prompt_workday_hours: [
        "🕐 How many hours per day does the business run? ({0}-{1})",
        "🕐 ¿Cuántas horas al día funciona el negocio? ({0}-{1})",
        "🕐 企业每天运行多少小时？({0}-{1})",
        "🕐 Wie viele Stunden pro Tag läuft das Geschäft? ({0}-{1})",
        "🕐 Сколько часов в день работает бизнес? ({0}-{1})",
        "🕐 Combien d'heures par jour le commerce fonctionne-t-il ? ({0}-{1})",
    ],
    validate_workday_range: [
        "Enter a number between {0} and {1}",
        "Introduce un número entre {0} y {1}",
        "请输入 {0} 到 {1} 之间的数字",
        "Gib eine Zahl zwischen {0} und {1} ein",
        "Введите число от {0} до {1}",
        "Saisissez un nombre entre {0} et {1}",
    ],
    err_read_workday: [
        "could not read workday hours: {0}",
        "no se pudo leer las horas de jornada: {0}",
        "无法读取工作日小时数: {0}",
        "Arbeitstunden konnten nicht gelesen werden: {0}",
        "не удалось прочитать часы рабочего дня: {0}",
        "impossible de lire les heures de travail : {0}",
    ],
    prompt_parallel_products: [
        "🧵 How many parallel products can the business produce? ({0}-{1})",
        "🧵 ¿Cuántos productos en paralelo puede fabricar el negocio? ({0}-{1})",
        "🧵 企业能同时生产多少个并行产品？({0}-{1})",
        "🧵 Wie viele Parallelprodukte kann das Geschäft herstellen? ({0}-{1})",
        "🧵 Сколько параллельных продуктов может производить бизнес? ({0}-{1})",
        "🧵 Combien de produits parallèles l'entreprise peut-elle produire ? ({0}-{1})",
    ],
    validate_parallel_range: [
        "Enter a number between {0} and {1}",
        "Introduce un número entre {0} y {1}",
        "请输入 {0} 到 {1} 之间的数字",
        "Gib eine Zahl zwischen {0} und {1} ein",
        "Введите число от {0} до {1}",
        "Saisissez un nombre entre {0} et {1}",
    ],
    err_read_parallel: [
        "could not read number of parallel products: {0}",
        "no se pudo leer el número de productos en paralelo: {0}",
        "无法读取并行产品数量: {0}",
        "Anzahl der Parallelprodukte konnte nicht gelesen werden: {0}",
        "не удалось прочитать количество параллельных продуктов: {0}",
        "impossible de lire le nombre de produits parallèles : {0}",
    ],
    sales_needed_header: [
        "➡️ Required sales to reach the net profit:",
        "➡️ Ventas necesarias para alcanzar el beneficio neto:",
        "➡️ 达到净利润所需的销售：",
        "➡️ Erforderliche Verkäufe zum Erreichen des Nettogewinns:",
        "➡️ Требуемые продажи для достижения чистой прибыли:",
        "➡️ Ventes requises pour atteindre le profit net :",
    ],
    monthly_label: [
        "📆 Monthly ({0} {1}):\t{2} sales",
        "📆 Mensual ({0} {1}):\t{2} ventas",
        "📆 月度 ({0} {1}):\t{2} 次销售",
        "📆 Monatlich ({0} {1}):\t{2} Verkäufe",
        "📆 Ежемесячно ({0} {1}):\t{2} продаж",
        "📆 Mensuel ({0} {1}) :\t{2} ventes",
    ],
    annual_label: [
        "📅 Annual ({0} {1}):\t{2} sales",
        "📅 Anual ({0} {1}):\t{2} ventas",
        "📅 年度 ({0} {1}):\t{2} 次销售",
        "📅 Jährlich ({0} {1}):\t{2} Verkäufe",
        "📅 Ежегодно ({0} {1}):\t{2} продаж",
        "📅 Annuel ({0} {1}) :\t{2} ventes",
    ],
    err_write_result_file: [
        "could not write the result file: {0}",
        "no se pudo escribir el archivo de resultados: {0}",
        "无法写入结果文件: {0}",
        "Ergebnisdatei konnte nicht geschrieben werden: {0}",
        "не удалось записать файл результатов: {0}",
        "impossible d'écrire le fichier de résultats : {0}",
    ],

    // --- simulator.rs : multi-product split ----------------------------------
    warn_product_excluded: [
        "⚠️ «{0}»: net profit {1} {2} ≤ 0, excluded from the split.",
        "⚠️ «{0}»: beneficio neto {1} {2} ≤ 0, se excluye del reparto.",
        "⚠️ «{0}»: 净利润 {1} {2} ≤ 0，已从分配中排除。",
        "⚠️ «{0}»: Nettogewinn {1} {2} ≤ 0, von der Aufteilung ausgeschlossen.",
        "⚠️ «{0}»: чистая прибыль {1} {2} ≤ 0, исключён из распределения.",
        "⚠️ «{0} » : profit net {1} {2} ≤ 0, exclu de la répartition.",
    ],
    warn_no_positive_products: [
        "⚠️ No product has positive net profit; the goal cannot be split.",
        "⚠️ Ningún producto tiene beneficio neto positivo; no se puede repartir la meta.",
        "⚠️ 没有产品具有正的净利润；无法分配目标。",
        "⚠️ Kein Produkt hat positiven Nettogewinn; das Ziel kann nicht aufgeteilt werden.",
        "⚠️ Ни один продукт не имеет положительной чистой прибыли; цель распределить нельзя.",
        "⚠️ Aucun produit n'a de profit net positif ; l'objectif ne peut pas être réparti.",
    ],
    random_split_header: [
        "🎲 Random sales split (1%–70% per product, normalized to 100%):",
        "🎲 Reparto aleatorio de ventas (1%–70% por producto, normalizado al 100%):",
        "🎲 随机销售分配（每个产品 1%–70%，归一化为 100%）：",
        "🎲 Zufällige Vertriebsaufteilung (1 %–70 % pro Produkt, auf 100 % normalisiert):",
        "🎲 Случайное распределение продаж (1%–70% на продукт, нормировано к 100%):",
        "🎲 Répartition aléatoire des ventes (1 %–70 % par produit, normalisée à 100 %) :",
    ],
    split_item: [
        "  • {0}  🎲 {1}%  → {2}% of the goal",
        "  • {0}  🎲 {1}%  → {2}% de la meta",
        "  • {0}  🎲 {1}%  → 占目标的 {2}%",
        "  • {0}  🎲 {1}%  → {2}% des Ziels",
        "  • {0}  🎲 {1}%  → {2}% от цели",
        "  • {0}  🎲 {1}%  → {2}% de l'objectif",
    ],
    per_product_header: [
        "➡️ Required sales per product (split goal):",
        "➡️ Ventas necesarias por producto (meta repartida):",
        "➡️ 每个产品所需的销售（分配目标）：",
        "➡️ Erforderliche Verkäufe pro Produkt (aufgeteiltes Ziel):",
        "➡️ Требуемые продажи по продуктам (распределённая цель):",
        "➡️ Ventes requises par produit (objectif réparti) :",
    ],
    per_product_monthly: [
        "      📆 Monthly:  {0} {1}  {2} sales  🕐 {3} min ({4} h)",
        "      📆 Mensual:  {0} {1}  {2} ventas  🕐 {3} min ({4} h)",
        "      📆 月度:  {0} {1}  {2} 次销售  🕐 {3} 分钟 ({4} 小时)",
        "      📆 Monatlich:  {0} {1}  {2} Verkäufe  🕐 {3} Min ({4} Std.)",
        "      📆 Месяц:  {0} {1}  {2} продаж  🕐 {3} мин ({4} ч)",
        "      📆 Mensuel :  {0} {1}  {2} ventes  🕐 {3} min ({4} h)",
    ],
    per_product_annual: [
        "      📅 Annual:   {0} {1}  {2} sales  🕐 {3} min ({4} h)",
        "      📅 Anual:    {0} {1}  {2} ventas  🕐 {3} min ({4} h)",
        "      📅 年度:      {0} {1}  {2} 次销售  🕐 {3} 分钟 ({4} 小时)",
        "      📅 Jährlich: {0} {1}  {2} Verkäufe  🕐 {3} Min ({4} Std.)",
        "      📅 Год:      {0} {1}  {2} продаж  🕐 {3} мин ({4} ч)",
        "      📅 Annuel :  {0} {1}  {2} ventes  🕐 {3} min ({4} h)",
    ],
    totals_header: [
        "📊 Totals ({0} products):",
        "📊 Totales ({0} productos):",
        "📊 合计（{0} 个产品）：",
        "📊 Gesamtergebnis ({0} Produkte):",
        "📊 Итоги ({0} продуктов):",
        "📊 Totaux ({0} produits) :",
    ],
    total_monthly_sales: [
        "  📆 Total monthly sales:  {0}  🕐 {1} min ({2} h) ({3} parallel products in {4} workday hours) ({5} workdays)",
        "  📆 Ventas mensuales totales:  {0}  🕐 {1} min ({2} h) ({3} productos en paralelo en {4} horas de jornada) ({5} jornadas)",
        "  📆 月度总销售:  {0}  🕐 {1} 分钟 ({2} 小时) ({3} 个并行产品，{4} 工作小时) ({5} 工作日)",
        "  📆 Monatliche Gesamtverkäufe:  {0}  🕐 {1} Min ({2} Std.) ({3} Parallelprodukte in {4} Arbeitstunden) ({5} Arbeitstage)",
        "  📆 Всего продаж за месяц:  {0}  🕐 {1} мин ({2} ч) ({3} параллельных продукта при {4} часах рабочего дня) ({5} рабочих дней)",
        "  📆 Ventes mensuelles totales :  {0}  🕐 {1} min ({2} h) ({3} produits parallèles sur {4} heures de travail) ({5} jours de travail)",
    ],
    total_month_row: [
        "{0} sales  🕐 {1} min ({2} h)",
        "{0} ventas  🕐 {1} min ({2} h)",
        "{0} 次销售  🕐 {1} 分钟 ({2} 小时)",
        "{0} Verkäufe  🕐 {1} Min ({2} Std.)",
        "{0} продаж  🕐 {1} мин ({2} ч)",
        "{0} ventes  🕐 {1} min ({2} h)",
    ],
    total_annual_sales: [
        "  📅 Total annual sales:  {0}  🕐 {1} min ({2} h) ({3} parallel products in {4} workday hours) ({5} workdays)",
        "  📅 Ventas anuales totales:  {0}  🕐 {1} min ({2} h) ({3} productos en paralelo en {4} horas de jornada) ({5} jornadas)",
        "  📅 年度总销售:  {0}  🕐 {1} 分钟 ({2} 小时) ({3} 个并行产品，{4} 工作小时) ({5} 工作日)",
        "  📅 Jährliche Gesamtverkäufe:  {0}  🕐 {1} Min ({2} Std.) ({3} Parallelprodukte in {4} Arbeitstunden) ({5} Arbeitstage)",
        "  📅 Всего продаж за год:  {0}  🕐 {1} мин ({2} ч) ({3} параллельных продукта при {4} часах рабочего дня) ({5} рабочих дней)",
        "  📅 Ventes annuelles totales :  {0}  🕐 {1} min ({2} h) ({3} produits parallèles sur {4} heures de travail) ({5} jours de travail)",
    ],
    err_write_result_file_for: [
        "could not write the result file for «{0}»: {1}",
        "no se pudo escribir el archivo de resultados para «{0}»: {1}",
        "无法为 «{0}» 写入结果文件: {1}",
        "Ergebnisdatei für «{0}» konnte nicht geschrieben werden: {1}",
        "не удалось записать файл результатов для «{0}»: {1}",
        "impossible d'écrire le fichier de résultats pour « {0} » : {1}",
    ],
    err_write_totals_file: [
        "could not write the totals file: {0}",
        "no se pudo escribir el archivo de totales: {0}",
        "无法写入总计文件: {0}",
        "Gesamtdatei konnte nicht geschrieben werden: {0}",
        "не удалось записать файл итогов: {0}",
        "impossible d'écrire le fichier des totaux : {0}",
    ],

    // --- simulator.rs : menu + file selection --------------------------------
    warn_no_txt_files: [
        "⚠️ No valid .txt files in folder '{0}'",
        "⚠️ No hay archivos .txt válidos en la carpeta '{0}'",
        "⚠️ 文件夹 '{0}' 中没有有效的 .txt 文件",
        "⚠️ Keine gültigen .txt-Dateien im Ordner '{0}'",
        "⚠️ В папке '{0}' нет допустимых .txt-файлов",
        "⚠️ Aucun fichier .txt valide dans le dossier « {0} »",
    ],
    menu_prompt: [
        "📄 Select one or more files (space to mark, enter to confirm):",
        "📄 Selecciona uno o varios archivos (espacio para marcar, intro para confirmar):",
        "📄 选择一个或多个文件（空格标记，回车确认）：",
        "📄 Wähle eine oder mehrere Dateien (Leertaste zum Markieren, Enter zum Bestätigen):",
        "📄 Выберите один или несколько файлов (пробел — отметить, ввод — подтвердить):",
        "📄 Sélectionnez un ou plusieurs fichiers (espace pour cocher, entrée pour confirmer) :",
    ],
    err_menu: [
        "menu error: {0}",
        "error en el menú: {0}",
        "菜单错误: {0}",
        "Menüfehler: {0}",
        "ошибка меню: {0}",
        "erreur de menu : {0}",
    ],
    warn_no_selection: [
        "⚠️ No file was selected.",
        "⚠️ No se seleccionó ningún archivo.",
        "⚠️ 未选择任何文件。",
        "⚠️ Es wurde keine Datei ausgewählt.",
        "⚠️ Файл не выбран.",
        "⚠️ Aucun fichier sélectionné.",
    ],
    err_read_file: [
        "❌ Error reading file \"{0}\": {1}",
        "❌ Error al leer el archivo \"{0}\": {1}",
        "❌ 读取文件 \"{0}\" 时出错: {1}",
        "❌ Fehler beim Lesen der Datei \"{0}\": {1}",
        "❌ Ошибка чтения файла \"{0}\": {1}",
        "❌ Erreur de lecture du fichier « {0} » : {1}",
    ],

    // --- simulator.rs : one-line result summary ------------------------------
    summary_net_profit: [
        "\t💶 {0} {1}",
        "\t💶 {0} {1}",
        "\t💶 {0} {1}",
        "\t💶 {0} {1}",
        "\t💶 {0} {1}",
        "\t💶 {0} {1}",
    ],
    summary_margin: [
        "\t📊 {0}%",
        "\t📊 {0}%",
        "\t📊 {0}%",
        "\t📊 {0}%",
        "\t📊 {0}%",
        "\t📊 {0}%",
    ],
    summary_monthly: [
        "\t📆 {0} sales (month) \t 🕐 {1} min (month)",
        "\t📆 {0} ventas (mes) \t 🕐 {1} min (mes)",
        "\t📆 {0} 次销售（月）\t 🕐 {1} 分钟（月）",
        "\t📆 {0} Verkäufe (Monat) \t 🕐 {1} Min (Monat)",
        "\t📆 {0} продаж (месяц) \t 🕐 {1} мин (месяц)",
        "\t📆 {0} ventes (mois) \t 🕐 {1} min (mois)",
    ],
    summary_annual: [
        "\t📅 {0} sales (year) \t🕐 {1} hours (year)",
        "\t📅 {0} ventas (año) \t🕐 {1} horas (año)",
        "\t📅 {0} 次销售（年）\t🕐 {1} 小时（年）",
        "\t📅 {0} Verkäufe (Jahr) \t🕐 {1} Std. (Jahr)",
        "\t📅 {0} продаж (год) \t🕐 {1} часов (год)",
        "\t📅 {0} ventes (an) \t🕐 {1} heures (an)",
    ],

    // --- tui.rs : tabs, sidebar titles, slider labels ------------------------
    tui_tab_products: ["Products", "Productos", "产品", "Produkte", "Продукты", "Produits"],
    tui_tab_graph: ["Graph", "Gráfico", "图表", "Diagramm", "График", "Graphique"],
    tui_sidebar_month: ["Month", "Mes", "月份", "Monat", "Месяц", "Mois"],
    tui_sidebar_settings: ["Settings", "Ajustes", "设置", "Einstellungen", "Настройки", "Paramètres"],
    tui_sidebar_totals: ["Totals", "Totales", "合计", "Gesamt", "Итоги", "Totaux"],
    tui_products_yearly: ["Products (yearly %)", "Productos (% anual)", "产品（年度%）", "Produkte (jährlich %)", "Продукты (годовой %)", "Produits (% annuel)"],
    tui_month_pct_sales: ["{0} (% sales)", "{0} (% ventas)", "{0} (销售%)", "{0} (Verkäufe %)", "{0} (продажи %)", "{0} (% ventes)"],
    tui_slider_workday: ["Workday hours", "Horas de jornada", "工作日小时", "Arbeitsstunden", "Часы рабочего дня", "Heures de travail"],
    tui_slider_parallel: ["Parallel products", "Productos en paralelo", "并行产品", "Parallelprodukte", "Параллельные продукты", "Produits parallèles"],
    tui_slider_monthly_goal: ["Monthly net-profit goal", "Meta mensual de beneficio neto", "月度净利润目标", "Monatliches Nettogewinnziel", "Месячная цель по чистой прибыли", "Objectif mensuel de profit net"],
    tui_slider_yearly_goal: ["Yearly net-profit goal", "Meta anual de beneficio neto", "年度净利润目标", "Jährliches Nettogewinnziel", "Годовая цель по чистой прибыли", "Objectif annuel de profit net"],
    tui_slider_month: ["Month", "Mes", "月份", "Monat", "Месяц", "Mois"],
    tui_parallel_label: ["Parallel products [{0}..={1}]", "Productos en paralelo [{0}..={1}]", "并行产品 [{0}..={1}]", "Parallelprodukte [{0}..={1}]", "Параллельные продукты [{0}..={1}]", "Produits parallèles [{0}..={1}]"],
    tui_lock_year: [" lock year", " bloquear año", " 锁定年度", " Jahr sperren", " блок. год", " verrouiller année"],
    tui_lock_month: [" lock month", " bloquear mes", " 锁定月份", " Monat sperren", " блок. месяц", " verrouiller mois"],

    // --- tui.rs : chart legend / stats ---------------------------------------
    tui_yearly_sales: ["Yearly sales", "Ventas anuales", "年度销售", "Jährliche Verkäufe", "Годовые продажи", "Ventes annuelles"],
    tui_legend_units: ["\u{25a0} units (n)   ", "\u{25a0} unidades (n)   ", "\u{25a0} 单位 (n)   ", "\u{25a0} Einheiten (n)   ", "\u{25a0} единиц (n)   ", "\u{25a0} unités (n)   "],
    tui_legend_profit: ["\u{25a0} profit ($)   ", "\u{25a0} beneficio ($)   ", "\u{25a0} 利润 ($)   ", "\u{25a0} Gewinn ($)   ", "\u{25a0} прибыль ($)   ", "\u{25a0} profit ($)   "],
    tui_legend_cost: ["\u{25a0} cost ($)   ", "\u{25a0} coste ($)   ", "\u{25a0} 成本 ($)   ", "\u{25a0} Kosten ($)   ", "\u{25a0} затраты ($)   ", "\u{25a0} coût ($)   "],
    tui_axis_max: ["axis max", "máx. eje", "轴最大值", "Achsenmax", "макс. оси", "max. axe"],
    tui_max: ["max", "máx", "最大", "Max", "макс", "max"],
    tui_profit: ["profit", "beneficio", "利润", "Gewinn", "прибыль", "profit"],
    tui_yearly: ["yearly", "anual", "年度", "jährlich", "годовой", "annuel"],

    // --- tui.rs : donut captions ---------------------------------------------
    tui_donut_margin: ["margin", "margen", "利润率", "Marge", "маржа", "marge"],
    tui_donut_vs_year: ["vs year", "vs año", "vs 年度", "vs Jahr", "vs год", "vs an"],

    // --- tui.rs : totals column labels ---------------------------------------
    tui_label_monthly: ["Monthly", "Mensual", "月度", "Monatlich", "Ежемесячно", "Mensuel"],
    tui_label_yearly: ["Yearly", "Anual", "年度", "Jährlich", "Ежегодно", "Annuel"],
    tui_label_settings: ["Settings", "Ajustes", "设置", "Einstellungen", "Настройки", "Paramètres"],
    tui_label_sales: ["sales", "ventas", "销售", "Verkäufe", "продажи", "ventes"],
    tui_label_min: ["min", "min", "分钟", "Min", "мин", "min"],
    tui_label_hours: ["hours", "horas", "小时", "Std.", "часы", "heures"],
    tui_label_workdays: ["workdays", "jornadas", "工作日", "Arbeitstage", "раб. дни", "jours trav."],
    tui_label_workday: ["workday", "jornada", "工作日", "Arbeitstag", "раб. день", "jour trav."],
    tui_label_parallel: ["parallel", "paralelo", "并行", "Parallel", "паралл.", "parall."],
    tui_label_yearly_ref: ["Yearly ref", "Ref. anual", "年度参考", "Jahresref.", "Годовая ссылка", "Réf. annuelle"],
    tui_label_12x_mo: ["12x mo", "12x mes", "12x 月", "12x Mo", "12x мес", "12x mois"],
    tui_label_goal: ["goal", "meta", "目标", "Ziel", "цель", "objectif"],
    tui_suffix_hours: ["h", "h", "小时", "Std.", "ч", "h"],

    // --- tui.rs : footer / status / regions ----------------------------------
    tui_region_main: ["main", "principal", "主区域", "Hauptbereich", "основная", "principal"],
    tui_region_sidebar: ["sidebar", "lateral", "侧栏", "Seitenleiste", "боковая", "latéral"],
    tui_footer: [
        "region: {0}   Tab region   Shift+Tab tab   \u{2191}/\u{2193} scroll/navigate   \u{2190}/\u{2192} adjust   Space lock   Ctrl+E export   Ctrl+H help   q quit",
        "región: {0}   Tab región   Shift+Tab pestaña   \u{2191}/\u{2193} desplazar/navegar   \u{2190}/\u{2192} ajustar   Espacio bloquear   Ctrl+E exportar   Ctrl+H ayuda   q salir",
        "区域: {0}   Tab 区域   Shift+Tab 标签   \u{2191}/\u{2193} 滚动/导航   \u{2190}/\u{2192} 调整   空格 锁定   Ctrl+E 导出   Ctrl+H 帮助   q 退出",
        "Bereich: {0}   Tab Bereich   Shift+Tab Tab   \u{2191}/\u{2193} scrollen/navigieren   \u{2190}/\u{2192} anpassen   Leertaste sperren   Ctrl+E Export   Ctrl+H Hilfe   q Beenden",
        "регион: {0}   Tab регион   Shift+Tab вкладка   \u{2191}/\u{2193} прокрутка/навигация   \u{2190}/\u{2192} настроить   Пробел блок.   Ctrl+E экспорт   Ctrl+H справка   q выход",
        "région : {0}   Tab région   Shift+Tab onglet   \u{2191}/\u{2193} défiler/naviguer   \u{2190}/\u{2192} ajuster   Espace verrouiller   Ctrl+E exporter   Ctrl+H aide   q quitter",
    ],
    tui_footer_status: [
        "{0}   |   region: {1}   Tab region   Shift+Tab tab   \u{2191}/\u{2193} scroll/navigate   \u{2190}/\u{2192} adjust   Space lock   Ctrl+E export   Ctrl+H help   q quit",
        "{0}   |   región: {1}   Tab región   Shift+Tab pestaña   \u{2191}/\u{2193} desplazar/navegar   \u{2190}/\u{2192} ajustar   Espacio bloquear   Ctrl+E exportar   Ctrl+H ayuda   q salir",
        "{0}   |   区域: {1}   Tab 区域   Shift+Tab 标签   \u{2191}/\u{2193} 滚动/导航   \u{2190}/\u{2192} 调整   空格 锁定   Ctrl+E 导出   Ctrl+H 帮助   q 退出",
        "{0}   |   Bereich: {1}   Tab Bereich   Shift+Tab Tab   \u{2191}/\u{2193} scrollen/navigieren   \u{2190}/\u{2192} anpassen   Leertaste sperren   Ctrl+E Export   Ctrl+H Hilfe   q Beenden",
        "{0}   |   регион: {1}   Tab регион   Shift+Tab вкладка   \u{2191}/\u{2193} прокрутка/навигация   \u{2190}/\u{2192} настроить   Пробел блок.   Ctrl+E экспорт   Ctrl+H справка   q выход",
        "{0}   |   région : {1}   Tab région   Shift+Tab onglet   \u{2191}/\u{2193} défiler/naviguer   \u{2190}/\u{2192} ajuster   Espace verrouiller   Ctrl+E exporter   Ctrl+H aide   q quitter",
    ],
    tui_export_error: ["export error ({0}): {1}", "error de exportación ({0}): {1}", "导出错误 ({0}): {1}", "Exportfehler ({0}): {1}", "ошибка экспорта ({0}): {1}", "erreur d'export ({0}) : {1}"],
    tui_export_error_totals: ["export error (totals): {0}", "error de exportación (totales): {0}", "导出错误 (合计): {0}", "Exportfehler (Gesamt): {0}", "ошибка экспорта (итоги): {0}", "erreur d'export (totaux) : {0}"],
    tui_exported: ["exported {0} product files + totals to {1}", "exportados {0} archivos de producto + totales a {1}", "已导出 {0} 个产品文件及合计到 {1}", "{0} Produkdateien + Gesamt exportiert nach {1}", "экспортировано {0} файлов продуктов + итоги в {1}", "exporté {0} fichiers produit + totaux vers {1}"],
    tui_no_products: ["No products with a positive net profit were found in {0}", "No se encontraron productos con beneficio neto positivo en {0}", "在 {0} 中未找到具有正净利润的产品", "Keine Produkte mit positivem Nettogewinn in {0} gefunden", "В {0} не найдено продуктов с положительной чистой прибылью", "Aucun produit avec un profit net positif trouvé dans {0}"],

    // --- tui.rs : help screen (Ctrl+H) ---------------------------------------
    tui_help_title: ["Help", "Ayuda", "帮助", "Hilfe", "Справка", "Aide"],
    tui_help_text: [
        "## Product definition (.txt files)\nEach product is a plain-text file in the business folder. The first line defines the product; every following line starting with \"- \" is a cost.\n\n  + <name> : <sale_price> <currency> : <production_time> <unit>\n    - <cost_price> <currency> <description>\n    - <cost_price> <currency> <description>\n\n  Example:\n  + Beer : 2.7 USD : 0.2 mins\n    - 0.27 USD labor\n    - 0.32 USD beer\n    - 0.10 USD cleaning\n\n  - Currency: any 3-letter ISO 4217 code (USD, EUR, GBP, JPY, MXN, CNY, ...).\n  - Production-time units: \"mins\" or \"hours\".\n  - Products whose net profit (price - total cost) is zero or negative are skipped.\n  - Files matching *.simulation_results.txt and the hidden .simulation_state are ignored by the loader.\n\n## Monthly & yearly net-profit goals\nThe simulator targets a NET-PROFIT goal (income minus costs), not revenue. Two sidebar sliders set the targets:\n\n  - Monthly goal: the net profit each month must reach (default 1000).\n  - Yearly goal: a reference target shown next to the 12 x monthly sum (default 12000). The yearly total is the SUM of the 12 monthly results, so the yearly goal is only a reference.\n\nEach month, the monthly goal is split across products by their sales-% share, giving a required sales count per product:\n\n  required_sales = ceil(share * monthly_goal / net_profit_per_unit)\n\n## Parallel products / services\nThe \"parallel\" slider is how many products can be produced or services delivered at the same time. Together with the workday hours it defines the monthly production CAPACITY in minutes:\n\n  capacity_minutes = workday_hours * 22 workdays * 60 * parallel\n\nIf the required production minutes exceed capacity, sales are scaled down to fit (the goal cannot be fully met that month). The parallel range is automatically clamped so the goals stay reachable; the slider's own min/max is recomputed on every change.\n\n## Workday hours\nHow many hours per day the business operates (1..24, default 8). It feeds the capacity formula above and the \"workdays\" figure shown in the results:\n\n  workdays = required_hours / (workday_hours * parallel)\n\n## Monthly / yearly sales distribution %\nEach month's goal is divided between products using percentages. Every month column always sums to exactly 100%.\n\n  - Graph tab: a Month selector picks the month (Jan..Dec). Each product has a monthly-% slider. Editing it sets that product's % and redistributes the remainder EQUALLY across all other non-locked products in that month.\n  - Products tab: each product shows a yearly-% slider (the mean of its 12 monthly values). Editing it propagates the target to every month where the product isn't month-locked, redistributing within each month.\n\nThe chart draws 12 separate monthly columns, so the mix can vary per month (seasonal demand). The yearly total is the sum of the 12 months.\n\n## Locks\nLocks freeze a product's percentage so it is excluded from redistribution.\n\n  - Yearly lock (Products tab, Space on a yearly slider): freezes the product in ALL 12 months. Month checkboxes render checked and greyed out.\n  - Month lock (Graph tab, Space on a monthly slider): freezes the product only for the selected month (disabled if the product is yearly-locked).\n\nLocked products keep their fixed share of the 100% pie; the remaining percentage is split among the unlocked products.\n\n## Exports & state\nPress Ctrl+E to export the current simulation:\n\n  - One <product>.simulation_results.txt per product (stats + 12 monthly rows + annual row + workday/parallel).\n  - A totals.simulation_results.txt aggregating all products.\n  - A hidden .simulation_state file saving percentages, locks and settings.\n\nReopening the app restores the saved distribution. If products were added or removed since the save, each month's percentages are re-normalized to sum to 100.\n\n## Keys\n  Tab          toggle sidebar <-> main area\n  Shift+Tab    switch Products <-> Graph tab\n  Up / Down    scroll main area or navigate sidebar sliders\n  Left / Right adjust the focused slider\n  Space        toggle the lock of the focused product slider\n  Ctrl+E       export simulation + save state\n  Ctrl+H       open / close this help\n  q / Esc      quit (close help if it is open)",
        "## Definición de producto (archivos .txt)\nCada producto es un archivo de texto plano en la carpeta del negocio. La primera línea define el producto; cada línea siguiente que empiece por \"- \" es un coste.\n\n  + <nombre> : <precio_venta> <moneda> : <tiempo_producción> <unidad>\n    - <precio_coste> <moneda> <descripción>\n    - <precio_coste> <moneda> <descripción>\n\n  Ejemplo:\n  + Beer : 2.7 USD : 0.2 mins\n    - 0.27 USD labor\n    - 0.32 USD beer\n    - 0.10 USD cleaning\n\n  - Moneda: cualquier código ISO 4217 de 3 letras (USD, EUR, GBP, JPY, MXN, CNY, ...).\n  - Unidades de tiempo de producción: \"mins\" u \"hours\".\n  - Los productos cuyo beneficio neto (precio - coste total) sea cero o negativo se omiten.\n  - Los archivos *.simulation_results.txt y el oculto .simulation_state los ignora el cargador.\n\n## Objetivos de beneficio neto mensual y anual\nEl simulador persigue un objetivo de BENEFICIO NETO (ingresos menos costes), no de ingresos. Dos deslizadores de la barra lateral fijan los objetivos:\n\n  - Objetivo mensual: el beneficio neto que cada mes debe alcanzar (por defecto 1000).\n  - Objetivo anual: un objetivo de referencia mostrado junto a la suma de 12 x mensual (por defecto 12000). El total anual es la SUMA de los 12 resultados mensuales, por lo que el objetivo anual es solo una referencia.\n\nCada mes, el objetivo mensual se reparte entre los productos según su porcentaje de ventas, dando un número de ventas requeridas por producto:\n\n  required_sales = ceil(parte * objetivo_mensual / beneficio_neto_por_unidad)\n\n## Productos / servicios paralelos\nEl deslizador \"parallel\" indica cuántos productos pueden producirse o servicios entregarse a la vez. Junto con las horas de jornada define la CAPACIDAD de producción mensual en minutos:\n\n  capacity_minutes = horas_jornada * 22 jornadas * 60 * paralelo\n\nSi los minutos de producción requeridos superan la capacidad, las ventas se reducen para encajar (el objetivo no se puede cumplir ese mes). El rango de paralelo se ajusta automáticamente para que los objetivos sean alcanzables; el mínimo/máximo del deslizador se recalcula en cada cambio.\n\n## Horas de jornada\nCuántas horas al día opera el negocio (1..24, por defecto 8). Interviene en la fórmula de capacidad y en la figura de \"jornadas\" mostrada en los resultados:\n\n  workdays = horas_requeridas / (horas_jornada * paralelo)\n\n## Distribución de ventas mensual / anual %\nEl objetivo de cada mes se reparte entre los productos usando porcentajes. Cada columna de mes siempre suma exactamente 100%.\n\n  - Pestaña Gráfico: un selector de mes elige el mes (Ene..Dic). Cada producto tiene un deslizador de % mensual. Al editarlo se fija el % de ese producto y el resto se redistribuye EQUITATIVAMENTE entre los demás productos no bloqueados de ese mes.\n  - Pestaña Productos: cada producto muestra un deslizador de % anual (la media de sus 12 valores mensuales). Al editarlo se propaga el objetivo a cada mes en que el producto no esté bloqueado por mes, redistribuyendo dentro de cada mes.\n\nEl gráfico dibuja 12 columnas mensuales independientes, por lo que la mezcla puede variar por mes (demanda estacional). El total anual es la suma de los 12 meses.\n\n## Bloqueos\nLos bloqueos congelan el porcentaje de un producto para excluirlo de la redistribución.\n\n  - Bloqueo anual (pestaña Productos, Espacio en un deslizador anual): congela el producto en TODOS los 12 meses. Las casillas mensuales se muestran marcadas y en gris.\n  - Bloqueo mensual (pestaña Gráfico, Espacio en un deslizador mensual): congela el producto solo para el mes seleccionado (se desactiva si el producto está bloqueado anualmente).\n\nLos productos bloqueados mantienen su parte fija del pastel del 100%; el porcentaje restante se reparte entre los productos desbloqueados.\n\n## Exportación y estado\nPulsa Ctrl+E para exportar la simulación actual:\n\n  - Un <producto>.simulation_results.txt por producto (estadísticas + 12 filas mensuales + fila anual + jornada/paralelo).\n  - Un totals.simulation_results.txt agregando todos los productos.\n  - Un archivo oculto .simulation_state que guarda porcentajes, bloqueos y ajustes.\n\nAl reabrir la app se restaura la distribución guardada. Si se añadieron o quitaron productos desde el guardado, los porcentajes de cada mes se renormalizan para sumar 100.\n\n## Teclas\n  Tab          alternar barra lateral <-> área principal\n  Shift+Tab    cambiar pestaña Productos <-> Gráfico\n  Up / Down    desplazar área principal o navegar deslizadores\n  Left / Right ajustar el deslizador en foco\n  Space        alternar el bloqueo del deslizador de producto en foco\n  Ctrl+E       exportar simulación + guardar estado\n  Ctrl+H       abrir / cerrar esta ayuda\n  q / Esc      salir (cierra la ayuda si está abierta)",
        "## 产品定义（.txt 文件）\n每个产品是业务文件夹中的一个纯文本文件。第一行定义产品；此后每个以 \"- \" 开头的行是一个成本项。\n\n  + <名称> : <售价> <货币> : <生产时间> <单位>\n    - <成本价> <货币> <描述>\n    - <成本价> <货币> <描述>\n\n  示例：\n  + Beer : 2.7 USD : 0.2 mins\n    - 0.27 USD labor\n    - 0.32 USD beer\n    - 0.10 USD cleaning\n\n  - 货币：任意 3 位 ISO 4217 代码（USD、EUR、GBP、JPY、MXN、CNY 等）。\n  - 生产时间单位：\"mins\" 或 \"hours\"。\n  - 净利润（售价 - 总成本）为零或负的产品会被跳过。\n  - 匹配 *.simulation_results.txt 的文件以及隐藏的 .simulation_state 会被加载器忽略。\n\n## 月度与年度净利润目标\n模拟器以净利润（收入减成本）为目标，而非收入。两个侧栏滑块设定目标：\n\n  - 月度目标：每月应达到的净利润（默认 1000）。\n  - 年度目标：显示在 12 x 月度合计旁的参考目标（默认 12000）。年度总额是 12 个月结果的求和，因此年度目标仅作参考。\n\n每月，月度目标按各产品的销售百分比分摊，得到每个产品所需的销售数量：\n\n  required_sales = ceil(份额 * 月度目标 / 单位净利润)\n\n## 并行产品 / 服务\n\"parallel\" 滑块表示可同时生产或交付的产品/服务数量。它与每日工时一起定义以分钟为单位的月度产能：\n\n  capacity_minutes = 每日工时 * 22 工作日 * 60 * 并行数\n\n若所需生产分钟数超过产能，销量会按比例缩减以适应（该月无法完全达成目标）。并行范围会被自动钳制以保持目标可达；滑块的最小/最大值在每次更改时重新计算。\n\n## 每日工时\n业务每天运营的小时数（1..24，默认 8）。它参与上方的产能公式，以及结果中显示的“工作日”数值：\n\n  workdays = 所需小时 / (每日工时 * 并行数)\n\n## 月度 / 年度销售分布 %\n每月目标通过百分比在产品之间分配。每个月份列始终精确求和为 100%。\n\n  - Graph 标签：月份选择器选择月份（1 月..12 月）。每个产品有一个月度 % 滑块。编辑它会设定该产品的 %，并将余数在当月其他未锁定的产品之间平均重新分配。\n  - Products 标签：每个产品显示年度 % 滑块（其 12 个月度值的均值）。编辑它会将目标传播到该产品未被月锁定的每个月，并在每个月内重新分配。\n\n图表绘制 12 个独立的月份列，因此各月组合可不同（季节性需求）。年度总额是 12 个月之和。\n\n## 锁定\n锁定会冻结某产品的百分比，使其不参与重新分配。\n\n  - 年度锁定（Products 标签，在年度滑块上按 Space）：在全部 12 个月冻结该产品。月份复选框显示为已勾选并变灰。\n  - 月度锁定（Graph 标签，在月度滑块上按 Space）：仅冻结所选月份中该产品（若产品已被年度锁定则禁用）。\n\n被锁定的产品保持其在 100% 饼图中的固定份额；剩余百分比在未锁定产品之间分配。\n\n## 导出与状态\n按 Ctrl+E 导出当前模拟：\n\n  - 每个产品一个 <产品>.simulation_results.txt（统计 + 12 行月度 + 年度行 + 工时/并行）。\n  - 一个汇总全部产品的 totals.simulation_results.txt。\n  - 一个保存百分比、锁定和设置的隐藏 .simulation_state 文件。\n\n重新打开应用会恢复已保存的分布。若自上次保存后产品有增减，每个月的百分比会被重新归一化为求和 100。\n\n## 按键\n  Tab          切换侧栏 <-> 主区域\n  Shift+Tab    切换 Products <-> Graph 标签\n  Up / Down    滚动主区域或导航侧栏滑块\n  Left / Right 调整聚焦滑块\n  Space        切换聚焦产品滑块的锁定\n  Ctrl+E       导出模拟 + 保存状态\n  Ctrl+H       打开 / 关闭本帮助\n  q / Esc      退出（若帮助已打开则先关闭帮助）",
        "## Produktdefinition (.txt-Dateien)\nJedes Produkt ist eine Klartextdatei im Geschäftordner. Die erste Zeile definiert das Produkt; jede weitere Zeile, die mit „- \" beginnt, ist ein Kostenpunkt.\n\n  + <Name> : <Verkaufspreis> <Währung> : <Produktionszeit> <Einheit>\n    - <Kostenpreis> <Währung> <Beschreibung>\n    - <Kostenpreis> <Währung> <Beschreibung>\n\n  Beispiel:\n  + Beer : 2.7 USD : 0.2 mins\n    - 0.27 USD labor\n    - 0.32 USD beer\n    - 0.10 USD cleaning\n\n  - Währung: jeder 3-stellige ISO-4217-Code (USD, EUR, GBP, JPY, MXN, CNY, ...).\n  - Einheiten der Produktionszeit: „mins\" oder „hours\".\n  - Produkte mit null oder negativem Nettogewinn (Preis - Gesamtkosten) werden übersprungen.\n  - Dateien mit *.simulation_results.txt und die versteckte .simulation_state werden vom Loader ignoriert.\n\n## Monats- & Jahres-Nettogewinnziele\nDer Simulator verfolgt ein NETTOGEWINN-Ziel (Einnahmen minus Kosten), nicht Umsatz. Zwei Sidebar-Regler legen die Ziele fest:\n\n  - Monatsziel: der Nettogewinn, den jeder Monat erreichen soll (Standard 1000).\n  - Jahresziel: ein Referenzziel, neben der 12 x Monats-Summe angezeigt (Standard 12000). Die Jahressumme ist die SUMME der 12 Monatswerte, das Jahresziel ist also nur ein Referenzwert.\n\nJeden Monat wird das Monatsziel nach den Verkaufs-Prozenten der Produkte aufgeteilt; ergibt die erforderliche Stückzahl je Produkt:\n\n  required_sales = ceil(Anteil * Monatsziel / Nettogewinn_pro_Einheit)\n\n## Parallele Produkte / Dienste\nDer Regler „parallel\" gibt an, wie viele Produkte gleichzeitig hergestellt bzw. Dienste geliefert werden können. Zusammen mit den Arbeitsstunden ergibt sich die monatliche Produktions-KAPAZITÄT in Minuten:\n\n  capacity_minutes = arbeitsstunden * 22 Arbeitstage * 60 * parallel\n\nÜbersteigen die erforderlichen Produktionsminuten die Kapazität, werden die Verkäufe passend herunterskaliert (das Ziel ist in diesem Monat nicht vollständig erreichbar). Der Parallel-Bereich wird automatisch so eingeschränkt, dass die Ziele erreichbar bleiben; Min/Max des Reglers werden bei jeder Änderung neu berechnet.\n\n## Arbeitsstunden\nWie viele Stunden pro Tag das Geschäft arbeitet (1..24, Standard 8). Fließt in die Kapazitätsformel ein und in die „Arbeitstage\"-Angabe der Ergebnisse:\n\n  workdays = erforderliche_Stunden / (arbeitsstunden * parallel)\n\n## Monats- / Jahres-Verkaufsverteilung %\nDas Monatsziel wird über Prozentangaben unter die Produkte verteilt. Jede Monatsspalte ergibt immer exakt 100%.\n\n  - Graph-Tab: ein Monatswähler wählt den Monat (Jan..Dez). Jedes Produkt hat einen Monats-%-Regler. Bearbeiten setzt den % des Produkts und verteilt den Rest GLEICHMÄSSIG unter den anderen nicht gesperrten Produkten dieses Monats.\n  - Products-Tab: jedes Produkt zeigt einen Jahres-%-Regler (den Mittel seiner 12 Monatswerte). Bearbeiten propagiert den Zielwert in jeden Monat, in dem das Produkt nicht monatsgesperrt ist, und verteilt innerhalb jedes Monats neu.\n\nDas Diagramm zeichnet 12 separate Monatsspalten, sodass die Zusammensetzung je Monat variieren kann (saisonale Nachfrage). Die Jahressumme ist die Summe der 12 Monate.\n\n## Sperren\nSperren frieren den Prozentsatz eines Produkts ein und nehmen es von der Neuverteilung aus.\n\n  - Jahressperre (Products-Tab, Leertaste auf Jahresregler): friert das Produkt in ALLEN 12 Monaten ein. Monats-Kästchen erscheinen markiert und ausgegraut.\n  - Monatssperre (Graph-Tab, Leertaste auf Monatsregler): friert das Produkt nur im gewählten Monat (deaktiviert, falls das Produkt jahresgesperrt ist).\n\nGesperrte Produkte behalten ihren festen Anteil am 100%-Kuchen; der restliche Prozentsatz wird unter den nicht gesperrten Produkten aufgeteilt.\n\n## Export & Status\nMit Ctrl+E den aktuellen Stand exportieren:\n\n  - Pro Produkt eine <produkt>.simulation_results.txt (Statistik + 12 Monatszeilen + Jahreszeile + Arbeitstag/Parallel).\n  - Eine totals.simulation_results.txt über alle Produkte.\n  - Eine versteckte .simulation_state-Datei, die Prozente, Sperren und Einstellungen speichert.\n\nBeim erneuten Öffnen wird die gespeicherte Verteilung wiederhergestellt. Wurden Produkte hinzugefügt oder entfernt, werden die Prozente je Monat neu normalisiert, sodass sie 100 ergeben.\n\n## Tasten\n  Tab          Sidebar <-> Hauptbereich wechseln\n  Shift+Tab    Products <-> Graph-Tab wechseln\n  Up / Down    Hauptbereich scrollen oder Sidebar-Regler navigieren\n  Left / Right fokussierten Regler anpassen\n  Space        Sperre des fokussierten Produktreglers umschalten\n  Ctrl+E       Simulation exportieren + Status speichern\n  Ctrl+H       diese Hilfe öffnen / schließen\n  q / Esc      beenden (Hilfe schließen, wenn offen)",
        "## Определение продукта (файлы .txt)\nКаждый продукт — текстовый файл в папке бизнеса. Первая строка задаёт продукт; каждая следующая строка, начинающаяся с «- », — это статья затрат.\n\n  + <название> : <цена_продажи> <валюта> : <время_производства> <единица>\n    - <цена_затрат> <валюта> <описание>\n    - <цена_затрат> <валюта> <описание>\n\n  Пример:\n  + Beer : 2.7 USD : 0.2 mins\n    - 0.27 USD labor\n    - 0.32 USD beer\n    - 0.10 USD cleaning\n\n  - Валюта: любой 3-буквенный код ISO 4217 (USD, EUR, GBP, JPY, MXN, CNY, ...).\n  - Единицы времени производства: «mins» или «hours».\n  - Продукты с нулевой или отрицательной чистой прибылью (цена - итог затрат) пропускаются.\n  - Файлы *.simulation_results.txt и скрытый .simulation_state игнорируются загрузчиком.\n\n## Цели по месячной и годовой чистой прибыли\nСимулятор ориентируется на цель по ЧИСТОЙ ПРИБЫЛИ (доход минус затраты), а не по выручке. Два ползунка на боковой панели задают цели:\n\n  - Месячная цель: чистая прибыль, которую должен достигать каждый месяц (по умолчанию 1000).\n  - Годовая цель: эталонная цель рядом с суммой 12 x месячных (по умолчанию 12000). Годовой итог — это СУММА 12 месячных результатов, поэтому годовая цель лишь ориентир.\n\nКаждый месяц месячная цель распределяется между продуктами по их процентам продаж, давая требуемое число продаж по каждому продукту:\n\n  required_sales = ceil(доля * месячная_цель / чистая_прибыль_за_единицу)\n\n## Параллельные продукты / услуги\nПолзунок «parallel» — сколько продуктов можно производить или услуг оказывать одновременно. Вместе с часами рабочего дня он задаёт месячную ПРОИЗВОДСТВЕННУЮ МОЩНОСТЬ в минутах:\n\n  capacity_minutes = часы_рабочего_дня * 22 рабочих_дня * 60 * parallel\n\nЕсли требуемые минуты производства превышают мощность, продажи пропорционально уменьшаются (в этом месяце цель недостижима полностью). Диапазон parallel автоматически ограничивается так, чтобы цели оставались достижимыми; min/max ползунка пересчитываются при каждом изменении.\n\n## Часы рабочего дня\nСколько часов в день работает бизнес (1..24, по умолчанию 8). Используется в формуле мощности выше и в показателе «рабочие дни» в результатах:\n\n  workdays = требуемые_часы / (часы_рабочего_дня * parallel)\n\n## Месячное / годовое распределение продаж %\nЦель каждого месяца делится между продуктами через проценты. Каждый столбец месяца всегда даёт в сумме ровно 100%.\n\n  - Вкладка Graph: выбор месяца (Янв..Дек). У каждого продукта ползунок месячного %. Изменение задаёт % продукта и перераспределяет остаток ПОРОВНУ между остальными незаблокированными продуктами этого месяца.\n  - Вкладка Products: у каждого продукта ползунок годового % (среднее его 12 месячных значений). Изменение распространяет целевое значение на каждый месяц, где продукт не заблокирован по месяцу, с перераспределением внутри каждого месяца.\n\nДиаграмма строит 12 отдельных месячных столбцов, поэтому состав может меняться по месяцам (сезонный спрос). Годовой итог — сумма 12 месяцев.\n\n## Блокировки\nБлокировки замораживают процент продукта, исключая его из перераспределения.\n\n  - Годовая блокировка (вкладка Products, Space на годовом ползунке): замораживает продукт во ВСЕХ 12 месяцах. Флажки месяцев отображаются отмеченными и серыми.\n  - Месячная блокировка (вкладка Graph, Space на месячном ползунке): замораживает продукт только в выбранном месяце (отключена, если продукт заблокирован по году).\n\nЗаблокированные продукты сохраняют фиксированную долю 100%-ного пирога; оставшийся процент делится между незаблокированными продуктами.\n\n## Экспорт и состояние\nНажмите Ctrl+E для экспорта текущей симуляции:\n\n  - По одному <продукт>.simulation_results.txt на продукт (статистика + 12 месячных строк + годовая строка + рабочий день/parallel).\n  - Файл totals.simulation_results.txt, объединяющий все продукты.\n  - Скрытый файл .simulation_state, сохраняющий проценты, блокировки и настройки.\n\nПри повторном открытии восстанавливается сохранённое распределение. Если продукты были добавлены или удалены, проценты каждого месяца перенормируются к сумме 100.\n\n## Клавиши\n  Tab          переключить боковую панель <-> главную область\n  Shift+Tab    переключить вкладку Products <-> Graph\n  Up / Down    прокрутка главной области или навигация по ползункам\n  Left / Right настроить активный ползунок\n  Space        переключить блокировку активного ползунка продукта\n  Ctrl+E       экспорт симуляции + сохранение состояния\n  Ctrl+H       открыть / закрыть эту справку\n  q / Esc      выход (закрыть справку, если она открыта)",
        "## Définition du produit (fichiers .txt)\nChaque produit est un fichier texte brut dans le dossier du commerce. La première ligne définit le produit ; chaque ligne suivante commençant par « - » est un coût.\n\n  + <nom> : <prix_vente> <monnaie> : <temps_production> <unité>\n    - <prix_coût> <monnaie> <description>\n    - <prix_coût> <monnaie> <description>\n\n  Exemple :\n  + Beer : 2.7 USD : 0.2 mins\n    - 0.27 USD labor\n    - 0.32 USD beer\n    - 0.10 USD cleaning\n\n  - Monnaie : tout code ISO 4217 à 3 lettres (USD, EUR, GBP, JPY, MXN, CNY, ...).\n  - Unités de temps de production : « mins » ou « hours ».\n  - Les produits dont le profit net (prix - coût total) est nul ou négatif sont ignorés.\n  - Les fichiers *.simulation_results.txt et le .simulation_state caché sont ignorés par le chargeur.\n\n## Objectifs de profit net mensuel et annuel\nLe simulateur vise un objectif de PROFIT NET (revenus moins coûts), pas de chiffre d'affaires. Deux curseurs de la barre latérale fixent les objectifs :\n\n  - Objectif mensuel : le profit net que chaque mois doit atteindre (1000 par défaut).\n  - Objectif annuel : un objectif de référence affiché à côté de la somme 12 x mensuelle (12000 par défaut). Le total annuel est la SOMME des 12 résultats mensuels, l'objectif annuel n'est donc qu'une référence.\n\nChaque mois, l'objectif mensuel est réparti entre les produits selon leur part de ventes en %, donnant un nombre de ventes requises par produit :\n\n  required_sales = ceil(part * objectif_mensuel / profit_net_par_unité)\n\n## Produits / services parallèles\nLe curseur « parallel » indique combien de produits peuvent être produits ou services livrés simultanément. Avec les heures de journée, il définit la CAPACITÉ de production mensuelle en minutes :\n\n  capacity_minutes = heures_journée * 22 jours * 60 * parallel\n\nSi les minutes de production requises dépassent la capacité, les ventes sont réduites en proportion (l'objectif ne peut être pleinement atteint ce mois-là). La plage de parallel est automatiquement bornée pour que les objectifs restent atteignables ; le min/max du curseur est recalculé à chaque modification.\n\n## Heures de journée\nNombre d'heures par jour pendant lesquelles le commerce fonctionne (1..24, 8 par défaut). Entre dans la formule de capacité ci-dessus et dans la valeur « journées » affichée dans les résultats :\n\n  workdays = heures_requises / (heures_journée * parallel)\n\n## Distribution des ventes mensuelle / annuelle %\nL'objectif de chaque mois est réparti entre les produits via des pourcentages. Chaque colonne de mois fait toujours exactement 100 % au total.\n\n  - Onglet Graph : un sélecteur de mois choisit le mois (janv.~déc.). Chaque produit a un curseur de % mensuel. Le modifier fixe le % du produit et redistribue le reste ÉGALEMENT entre les autres produits non verrouillés de ce mois.\n  - Onglet Products : chaque produit affiche un curseur de % annuel (la moyenne de ses 12 valeurs mensuelles). Le modifier propage la cible à chaque mois où le produit n'est pas verrouillé par mois, en redistribuant dans chaque mois.\n\nLe graphique dessine 12 colonnes mensuelles distinctes, donc la répartition peut varier par mois (demande saisonnière). Le total annuel est la somme des 12 mois.\n\n## Verrous\nLes verrous figent le pourcentage d'un produit pour l'exclure de la redistribution.\n\n  - Verrou annuel (onglet Products, Espace sur un curseur annuel) : fige le produit sur TOUS les 12 mois. Les cases mensuelles apparaissent cochées et grisées.\n  - Verrou mensuel (onglet Graph, Espace sur un curseur mensuel) : fige le produit seulement pour le mois sélectionné (désactivé si le produit est verrouillé annuellement).\n\nLes produits verrouillés gardent leur part fixe du camembert des 100 % ; le reste est réparti entre les produits non verrouillés.\n\n## Exportation et état\nAppuyez sur Ctrl+E pour exporter la simulation actuelle :\n\n  - Un <produit>.simulation_results.txt par produit (statistiques + 12 lignes mensuelles + ligne annuelle + journée/parallel).\n  - Un totals.simulation_results.txt agrégeant tous les produits.\n  - Un fichier caché .simulation_state sauvegardant les pourcentages, verrous et réglages.\n\nÀ la réouverture, la distribution sauvegardée est restaurée. Si des produits ont été ajoutés ou supprimés depuis la sauvegarde, les pourcentages de chaque mois sont renormalisés pour sommer à 100.\n\n## Touches\n  Tab          basculer barre latérale <-> zone principale\n  Shift+Tab    basculer l'onglet Products <-> Graph\n  Up / Down    défiler la zone principale ou naviguer les curseurs\n  Left / Right ajuster le curseur actif\n  Space        basculer le verrou du curseur produit actif\n  Ctrl+E       exporter la simulation + sauver l'état\n  Ctrl+H       ouvrir / fermer cette aide\n  q / Esc      quitter (ferme l'aide si elle est ouverte)",
    ],
}

// ---------------------------------------------------------------------------
// Language selector
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    Zh,
    De,
    Ru,
    Fr,
}

impl Lang {
    /// Parse a CLI language code (`en`, `es`, ...) into a [`Lang`].
    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            "zh" => Some(Lang::Zh),
            "de" => Some(Lang::De),
            "ru" => Some(Lang::Ru),
            "fr" => Some(Lang::Fr),
            _ => None,
        }
    }

    /// Canonical short code (`en`, `es`, ...).
    #[allow(dead_code)]
    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Zh => "zh",
            Lang::De => "de",
            Lang::Ru => "ru",
            Lang::Fr => "fr",
        }
    }

    /// The default language used when `--lang` is not supplied.
    pub const DEFAULT: Lang = Lang::En;

    /// Borrow this language's dictionary.
    pub fn dict(&self) -> &'static Dict {
        match self {
            Lang::En => &EN,
            Lang::Es => &ES,
            Lang::Zh => &ZH,
            Lang::De => &DE,
            Lang::Ru => &RU,
            Lang::Fr => &FR,
        }
    }

    /// Localized month abbreviations (12 entries, Jan..Dec).
    pub fn months_abbr(&self) -> [&'static str; 12] {
        match self {
            Lang::En => ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
            Lang::Es => ["Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic"],
            Lang::Zh => ["1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月"],
            Lang::De => ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"],
            Lang::Ru => ["Янв", "Фев", "Мар", "Апр", "Май", "Июн", "Июл", "Авг", "Сен", "Окт", "Ноя", "Дек"],
            Lang::Fr => ["janv", "févr", "mars", "avr", "mai", "juin", "juil", "août", "sept", "oct", "nov", "déc"],
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime template formatting
// ---------------------------------------------------------------------------

/// Render a `{n}`-placeholder template against a slice of pre-formatted
/// argument strings.
///
/// Placeholders are written as `{0}`, `{1}`, ... (single digit, 0-9). A bare
/// `{` with no following digit is emitted literally. Multibyte UTF-8 is safe:
/// only the ASCII byte `{` (0x7B) is treated specially, and it never appears
/// inside a multibyte sequence.
pub fn fmt(template: &str, args: &[&str]) -> String {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len() + 32);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if i + 2 < bytes.len() && bytes[i + 1].is_ascii_digit() && bytes[i + 2] == b'}' {
                let idx = (bytes[i + 1] - b'0') as usize;
                if idx < args.len() {
                    out.push_str(args[idx]);
                }
                i += 3;
            } else {
                out.push('{');
                i += 1;
            }
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'{' {
                i += 1;
            }
            // Slicing is safe: both bounds land on UTF-8 char boundaries
            // (start is either 0 or just after a `{`, and we stop only at `{`).
            out.push_str(&template[start..i]);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Label alignment helpers
// ---------------------------------------------------------------------------

/// Approximate display width of a string: 1 column per ASCII char, 2 columns
/// per non-ASCII char. This is correct for CJK and Cyrillic and treats every
/// emoji as a double-width cell (the convention used by the overwhelming
/// majority of modern terminals). Accented Latin letters are counted as 2,
/// which overestimates by 1 per accent — acceptable for label alignment.
pub fn str_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

/// Left-justify `s`, padding with spaces so its display width reaches `width`.
/// Strings already at or beyond `width` are returned unchanged.
pub fn pad_to(s: &str, width: usize) -> String {
    let w = str_width(s);
    if w >= width {
        return s.to_string();
    }
    let mut r = String::with_capacity(s.len() + (width - w));
    r.push_str(s);
    for _ in 0..(width - w) {
        r.push(' ');
    }
    r
}

/// The label portion of a template: everything before the first `{0}`
/// placeholder, with any trailing whitespace (spaces or tabs) stripped.
pub fn prefix_before_value(template: &str) -> &str {
    match template.find("{0}") {
        Some(i) => template[..i].trim_end(),
        None => template.trim_end(),
    }
}

/// Like [`fmt`], but the label prefix (text before `{0}`) is right-padded
/// with spaces to `label_width` columns, replacing any tabs/whitespace that
/// were baked into the template. This yields terminal-independent alignment
/// regardless of how the per-language label was authored.
pub fn fmt_aligned(template: &str, args: &[&str], label_width: usize) -> String {
    let idx = template.find("{0}").unwrap_or(0);
    let prefix = template[..idx].trim_end();
    let rest = &template[idx..];
    format!("{}{}", pad_to(prefix, label_width), fmt(rest, args))
}

/// Like [`fmt_aligned`] but with a caller-supplied prefix string (padded to
/// `label_width`) instead of extracting the prefix from the template. Used for
/// month rows where the prefix includes a dynamic month abbreviation that must
/// appear right next to the emoji, not pushed right by the label-width padding.
pub fn fmt_prefixed(template: &str, prefix: &str, args: &[&str], label_width: usize) -> String {
    format!("{}{}", pad_to(prefix, label_width), fmt(template, args))
}

/// Right-justify `s`, padding on the left with spaces so its display width
/// reaches `width`. Used to align numeric columns.
pub fn pad_left(s: &str, width: usize) -> String {
    let w = str_width(s);
    if w >= width {
        return s.to_string();
    }
    let mut r = String::with_capacity(s.len() + (width - w));
    for _ in 0..(width - w) {
        r.push(' ');
    }
    r.push_str(s);
    r
}

/// Render a block of rows that share a common column structure, right-aligning
/// each value column to the maximum display width across all rows for that
/// column, and space-padding the label prefix to `label_width`. This is what
/// makes numeric columns line up across lines even when values have very
/// different magnitudes (e.g. `254.24` next to `5084.75`).
///
/// Each entry is `(template, row)` where `row` is the list of pre-formatted
/// argument strings for the template's `{0}`..`{n}` placeholders.
pub fn fmt_block(rows: &[(&str, Vec<String>)], label_width: usize) -> Vec<String> {
    let n_cols = rows.iter().map(|(_, r)| r.len()).max().unwrap_or(0);
    let mut col_w = vec![0usize; n_cols];
    for (_, r) in rows {
        for (i, v) in r.iter().enumerate() {
            col_w[i] = col_w[i].max(str_width(v));
        }
    }
    rows.iter()
        .map(|(t, r)| {
            let padded: Vec<String> = r
                .iter()
                .enumerate()
                .map(|(i, v)| pad_left(v, col_w[i]))
                .collect();
            let refs: Vec<&str> = padded.iter().map(String::as_str).collect();
            fmt_aligned(t, &refs, label_width)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
        assert_eq!(str_width("Цена"), 8);         // Cyrillic: 2 per char
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
}
