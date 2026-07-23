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
        "invalid sale currency '{0}': must be one of USD, USD, CAD",
        "moneda de venta inválida '{0}': debe ser una de USD, USD, CAD",
        "无效的售出货币 '{0}'：必须是 USD、USD、CAD 之一",
        "ungültige Verkaufswährung '{0}': muss USD, USD oder CAD sein",
        "недопустимая валюта продажи '{0}': должна быть USD, USD или CAD",
        "monnaie de vente invalide « {0} » : doit être USD, USD ou CAD",
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
        "invalid cost currency '{0}': must be one of USD, USD, CAD",
        "moneda de coste inválida '{0}': debe ser una de USD, USD, CAD",
        "无效的成本货币 '{0}'：必须是 USD、USD、CAD 之一",
        "ungültige Kostenwährung '{0}': muss USD, USD oder CAD sein",
        "недопустимая валюта затрат '{0}': должна быть USD, USD или CAD",
        "monnaie de coût invalide « {0} » : doit être USD, USD ou CAD",
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
