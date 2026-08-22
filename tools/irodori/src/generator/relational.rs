use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::data::{Counts, Customer, Dataset, Event, Order, OrderItem, Product};
use super::{
    GeneratedFile, GeneratorConfig, banner, inserts, integer_bool, json, json_sql, metadata_json,
    national_quote, number, payload_json, sql_bool, sql_quote,
};

const CUSTOMER_COLUMNS: &[&str] = &[
    "id",
    "name",
    "email",
    "country_code",
    "tier",
    "credit_limit",
    "is_active",
    "signup_source",
    "created_at",
    "metadata",
];
const PRODUCT_COLUMNS: &[&str] = &[
    "id",
    "sku",
    "name",
    "category",
    "price",
    "weight_kg",
    "in_stock",
    "supplier",
    "released_on",
    "tags",
];
const ORDER_COLUMNS: &[&str] = &[
    "id",
    "customer_id",
    "status",
    "channel",
    "currency",
    "subtotal",
    "tax",
    "total",
    "ordered_at",
    "shipped_at",
    "note",
];
const ITEM_COLUMNS: &[&str] = &[
    "id",
    "order_id",
    "product_id",
    "quantity",
    "unit_price",
    "discount_rate",
    "line_total",
];
const EVENT_COLUMNS: &[&str] = &[
    "id",
    "customer_id",
    "event_type",
    "occurred_at",
    "session_id",
    "device",
    "duration_ms",
    "payload",
];

pub(super) fn emit(
    repository_root: &Path,
    data: &Dataset,
    counts: Counts,
    config: GeneratorConfig,
) -> Result<Vec<GeneratedFile>> {
    let sqlite = emit_standard(
        "SQLite and DuckDB",
        SQLITE_SCHEMA,
        "",
        data,
        counts,
        config,
        BoolStyle::Sql,
        None,
    );
    Ok(vec![
        GeneratedFile::new(
            "postgres/01_samples.sql",
            emit_postgres(repository_root, data, counts, config)?,
        ),
        GeneratedFile::new("mysql/01_samples.sql", emit_mysql(data, counts, config)),
        GeneratedFile::new("oracle/01_samples.sql", emit_oracle(data, counts, config)),
        GeneratedFile::new(
            "sqlserver/01_samples.sql",
            emit_sql_server(data, counts, config),
        ),
        GeneratedFile::new(
            "clickhouse/01_samples.sql",
            emit_standard(
                "ClickHouse",
                CLICKHOUSE_SCHEMA,
                "samples.",
                data,
                counts,
                config,
                BoolStyle::Sql,
                None,
            ),
        ),
        GeneratedFile::new("sqlite/01_samples.sql", sqlite.clone()),
        GeneratedFile::new("duckdb/01_samples.sql", sqlite),
    ])
}

#[derive(Clone, Copy)]
enum BoolStyle {
    Sql,
    Integer,
}

impl BoolStyle {
    fn render(self, value: bool) -> String {
        match self {
            Self::Sql => sql_bool(value),
            Self::Integer => integer_bool(value),
        }
    }
}

fn emit_postgres(
    repository_root: &Path,
    data: &Dataset,
    counts: Counts,
    config: GeneratorConfig,
) -> Result<String> {
    let demo_path = repository_root.join("generator/postgres-demo.sql");
    let demo = fs::read_to_string(&demo_path)
        .with_context(|| format!("cannot read static PostgreSQL demo {}", demo_path.display()))?;
    Ok(emit_standard(
        "PostgreSQL (also TimescaleDB, CockroachDB, YugabyteDB)",
        POSTGRES_SCHEMA,
        "",
        data,
        counts,
        config,
        BoolStyle::Sql,
        Some(&format!("{POSTGRES_VIEWS}\n{demo}")),
    ))
}

fn emit_mysql(data: &Dataset, counts: Counts, config: GeneratorConfig) -> String {
    emit_standard(
        "MySQL (also MariaDB, TiDB)",
        MYSQL_SCHEMA,
        "",
        data,
        counts,
        config,
        BoolStyle::Integer,
        Some(MYSQL_VIEWS),
    )
}

#[allow(clippy::too_many_arguments)]
fn emit_standard(
    engine: &str,
    schema: &str,
    table_prefix: &str,
    data: &Dataset,
    counts: Counts,
    config: GeneratorConfig,
    bool_style: BoolStyle,
    suffix: Option<&str>,
) -> String {
    let mut sections = vec![banner(engine, data, counts, config), schema.to_owned()];
    sections.push(inserts(
        &format!("{table_prefix}customers"),
        CUSTOMER_COLUMNS,
        &data.customers,
        100,
        |customer| standard_customer_values(customer, bool_style),
    ));
    sections.push(inserts(
        &format!("{table_prefix}products"),
        PRODUCT_COLUMNS,
        &data.products,
        100,
        |product| standard_product_values(product, bool_style),
    ));
    sections.push(inserts(
        &format!("{table_prefix}orders"),
        ORDER_COLUMNS,
        &data.orders,
        100,
        standard_order_values,
    ));
    sections.push(inserts(
        &format!("{table_prefix}order_items"),
        ITEM_COLUMNS,
        &data.order_items,
        100,
        standard_item_values,
    ));
    sections.push(inserts(
        &format!("{table_prefix}events"),
        EVENT_COLUMNS,
        &data.events,
        100,
        standard_event_values,
    ));
    if let Some(suffix) = suffix {
        sections.push(suffix.to_owned());
    }
    sections.join("\n")
}

fn standard_customer_values(customer: &Customer, bool_style: BoolStyle) -> Vec<String> {
    vec![
        customer.id.to_string(),
        sql_quote(Some(&customer.name)),
        sql_quote(Some(&customer.email)),
        sql_quote(Some(customer.country_code)),
        sql_quote(Some(customer.tier)),
        number(customer.credit_limit),
        bool_style.render(customer.is_active),
        sql_quote(Some(customer.signup_source)),
        sql_quote(Some(&customer.created_at)),
        sql_quote(Some(&metadata_json(&customer.metadata))),
    ]
}

fn standard_product_values(product: &Product, bool_style: BoolStyle) -> Vec<String> {
    vec![
        product.id.to_string(),
        sql_quote(Some(&product.sku)),
        sql_quote(Some(&product.name)),
        sql_quote(Some(product.category)),
        number(product.price),
        number(product.weight_kg),
        bool_style.render(product.in_stock),
        sql_quote(Some(product.supplier)),
        sql_quote(Some(&product.released_on)),
        json_sql(&product.tags),
    ]
}

fn standard_order_values(order: &Order) -> Vec<String> {
    vec![
        order.id.to_string(),
        order.customer_id.to_string(),
        sql_quote(Some(order.status)),
        sql_quote(Some(order.channel)),
        sql_quote(Some(order.currency)),
        number(order.subtotal),
        number(order.tax),
        number(order.total),
        sql_quote(Some(&order.ordered_at)),
        sql_quote(order.shipped_at.as_deref()),
        sql_quote(order.note),
    ]
}

fn standard_item_values(item: &OrderItem) -> Vec<String> {
    vec![
        item.id.to_string(),
        item.order_id.to_string(),
        item.product_id.to_string(),
        item.quantity.to_string(),
        number(item.unit_price),
        number(item.discount_rate),
        number(item.line_total),
    ]
}

fn standard_event_values(event: &Event) -> Vec<String> {
    vec![
        event.id.to_string(),
        event.customer_id.to_string(),
        sql_quote(Some(event.event_type)),
        sql_quote(Some(&event.occurred_at)),
        sql_quote(Some(&event.session_id)),
        sql_quote(Some(event.device)),
        event.duration_ms.to_string(),
        sql_quote(Some(&payload_json(&event.payload))),
    ]
}

fn emit_oracle(data: &Dataset, counts: Counts, config: GeneratorConfig) -> String {
    let mut sections = vec![
        banner("Oracle Database Free", data, counts, config),
        ORACLE_SCHEMA.to_owned(),
    ];
    sections.push(insert_all(
        "customers",
        CUSTOMER_COLUMNS,
        &data.customers,
        50,
        |customer| {
            vec![
                customer.id.to_string(),
                sql_quote(Some(&customer.name)),
                sql_quote(Some(&customer.email)),
                sql_quote(Some(customer.country_code)),
                sql_quote(Some(customer.tier)),
                number(customer.credit_limit),
                integer_bool(customer.is_active),
                sql_quote(Some(customer.signup_source)),
                oracle_timestamp(Some(&customer.created_at)),
                sql_quote(Some(&metadata_json(&customer.metadata))),
            ]
        },
    ));
    sections.push(insert_all(
        "products",
        PRODUCT_COLUMNS,
        &data.products,
        50,
        |product| {
            vec![
                product.id.to_string(),
                sql_quote(Some(&product.sku)),
                sql_quote(Some(&product.name)),
                sql_quote(Some(product.category)),
                number(product.price),
                number(product.weight_kg),
                integer_bool(product.in_stock),
                sql_quote(Some(product.supplier)),
                format!(
                    "TO_DATE({}, 'YYYY-MM-DD')",
                    sql_quote(Some(&product.released_on))
                ),
                json_sql(&product.tags),
            ]
        },
    ));
    sections.push(insert_all(
        "orders",
        ORDER_COLUMNS,
        &data.orders,
        50,
        |order| {
            vec![
                order.id.to_string(),
                order.customer_id.to_string(),
                sql_quote(Some(order.status)),
                sql_quote(Some(order.channel)),
                sql_quote(Some(order.currency)),
                number(order.subtotal),
                number(order.tax),
                number(order.total),
                oracle_timestamp(Some(&order.ordered_at)),
                oracle_timestamp(order.shipped_at.as_deref()),
                sql_quote(order.note),
            ]
        },
    ));
    sections.push(insert_all(
        "order_items",
        ITEM_COLUMNS,
        &data.order_items,
        50,
        standard_item_values,
    ));
    sections.push(insert_all(
        "events",
        EVENT_COLUMNS,
        &data.events,
        50,
        |event| {
            vec![
                event.id.to_string(),
                event.customer_id.to_string(),
                sql_quote(Some(event.event_type)),
                oracle_timestamp(Some(&event.occurred_at)),
                sql_quote(Some(&event.session_id)),
                sql_quote(Some(event.device)),
                event.duration_ms.to_string(),
                sql_quote(Some(&payload_json(&event.payload))),
            ]
        },
    ));
    sections.push("\nCOMMIT;\nEXIT;\n".to_owned());
    sections.join("\n")
}

fn oracle_timestamp(value: Option<&str>) -> String {
    value.map_or_else(
        || "NULL".to_owned(),
        |value| {
            format!(
                "TO_TIMESTAMP({}, 'YYYY-MM-DD HH24:MI:SS')",
                sql_quote(Some(value))
            )
        },
    )
}

fn insert_all<T>(
    table: &str,
    columns: &[&str],
    rows: &[T],
    batch_size: usize,
    values: impl Fn(&T) -> Vec<String>,
) -> String {
    rows.chunks(batch_size)
        .map(|chunk| {
            let into = chunk
                .iter()
                .map(|row| {
                    format!(
                        "  INTO {table} ({}) VALUES ({})",
                        columns.join(", "),
                        values(row).join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("INSERT ALL\n{into}\nSELECT * FROM dual;")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn emit_sql_server(data: &Dataset, counts: Counts, config: GeneratorConfig) -> String {
    let mut sections = vec![
        banner("SQL Server", data, counts, config),
        SQL_SERVER_SCHEMA.to_owned(),
    ];
    sections.push(inserts(
        "customers",
        CUSTOMER_COLUMNS,
        &data.customers,
        100,
        |customer| {
            vec![
                customer.id.to_string(),
                national_quote(Some(&customer.name)),
                national_quote(Some(&customer.email)),
                sql_quote(Some(customer.country_code)),
                sql_quote(Some(customer.tier)),
                number(customer.credit_limit),
                integer_bool(customer.is_active),
                sql_quote(Some(customer.signup_source)),
                sql_quote(Some(&customer.created_at)),
                national_quote(Some(&metadata_json(&customer.metadata))),
            ]
        },
    ));
    sections.push(inserts(
        "products",
        PRODUCT_COLUMNS,
        &data.products,
        100,
        |product| {
            vec![
                product.id.to_string(),
                sql_quote(Some(&product.sku)),
                national_quote(Some(&product.name)),
                sql_quote(Some(product.category)),
                number(product.price),
                number(product.weight_kg),
                integer_bool(product.in_stock),
                national_quote(Some(product.supplier)),
                sql_quote(Some(&product.released_on)),
                national_quote(Some(&json(&product.tags))),
            ]
        },
    ));
    sections.push(inserts(
        "orders",
        ORDER_COLUMNS,
        &data.orders,
        100,
        |order| {
            let mut values = standard_order_values(order);
            values[10] = national_quote(order.note);
            values
        },
    ));
    sections.push(inserts(
        "order_items",
        ITEM_COLUMNS,
        &data.order_items,
        100,
        standard_item_values,
    ));
    sections.push(inserts(
        "events",
        EVENT_COLUMNS,
        &data.events,
        100,
        |event| {
            let mut values = standard_event_values(event);
            values[7] = national_quote(Some(&payload_json(&event.payload)));
            values
        },
    ));
    sections.push("GO\n".to_owned());
    sections.join("\n")
}

// Schema strings deliberately keep the formatting of the committed fixtures.
// Their data sections are shared above; only dialect-specific DDL lives here.

const POSTGRES_SCHEMA: &str = r#"
DROP VIEW  IF EXISTS top_products;
DROP VIEW  IF EXISTS customer_lifetime_value;
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS customers;
DROP TYPE  IF EXISTS order_status;
DROP TYPE  IF EXISTS customer_tier;

CREATE TYPE customer_tier AS ENUM ('bronze', 'silver', 'gold', 'platinum');
CREATE TYPE order_status  AS ENUM ('pending', 'processing', 'shipped', 'delivered', 'cancelled', 'refunded');

CREATE TABLE customers (
  id            integer        PRIMARY KEY,
  name          text           NOT NULL,
  email         text           NOT NULL UNIQUE,
  country_code  char(2)        NOT NULL,
  tier          customer_tier  NOT NULL,
  credit_limit  numeric(14, 2) NOT NULL CHECK (credit_limit >= 0),
  is_active     boolean        NOT NULL,
  signup_source text           NOT NULL,
  created_at    timestamptz    NOT NULL,
  metadata      jsonb          NOT NULL
);

CREATE TABLE products (
  id          integer        PRIMARY KEY,
  sku         text           NOT NULL UNIQUE,
  name        text           NOT NULL,
  category    text           NOT NULL,
  price       numeric(12, 2) NOT NULL CHECK (price >= 0),
  weight_kg   numeric(8, 3)  NOT NULL,
  in_stock    boolean        NOT NULL,
  supplier    text           NOT NULL,
  released_on date           NOT NULL,
  tags        jsonb          NOT NULL
);

CREATE TABLE orders (
  id          integer        PRIMARY KEY,
  customer_id integer        NOT NULL REFERENCES customers (id),
  status      order_status   NOT NULL,
  channel     text           NOT NULL,
  currency    char(3)        NOT NULL,
  subtotal    numeric(14, 2) NOT NULL,
  tax         numeric(14, 2) NOT NULL,
  total       numeric(14, 2) NOT NULL,
  ordered_at  timestamptz    NOT NULL,
  shipped_at  timestamptz,
  note        text,
  CONSTRAINT orders_ship_after_order CHECK (shipped_at IS NULL OR shipped_at >= ordered_at)
);

CREATE TABLE order_items (
  id            integer        PRIMARY KEY,
  order_id      integer        NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
  product_id    integer        NOT NULL REFERENCES products (id),
  quantity      integer        NOT NULL CHECK (quantity > 0),
  unit_price    numeric(12, 2) NOT NULL,
  discount_rate numeric(4, 2)  NOT NULL,
  line_total    numeric(14, 2) NOT NULL
);

CREATE TABLE events (
  id          integer     PRIMARY KEY,
  customer_id integer     NOT NULL REFERENCES customers (id),
  event_type  text        NOT NULL,
  occurred_at timestamptz NOT NULL,
  session_id  text        NOT NULL,
  device      text        NOT NULL,
  duration_ms integer     NOT NULL,
  payload     jsonb       NOT NULL
);

CREATE INDEX idx_customers_tier    ON customers (tier);
CREATE INDEX idx_orders_customer   ON orders (customer_id);
CREATE INDEX idx_orders_ordered_at ON orders (ordered_at DESC);
CREATE INDEX idx_items_order       ON order_items (order_id);
CREATE INDEX idx_events_customer   ON events (customer_id, occurred_at DESC);

COMMENT ON TABLE  customers          IS 'Registered buyers. One row per account.';
COMMENT ON COLUMN orders.shipped_at  IS 'NULL until the order ships.';
COMMENT ON TABLE  order_items        IS 'Order lines. subtotal on the order is their exact sum.';
"#;
const POSTGRES_VIEWS: &str = r#"
CREATE VIEW customer_lifetime_value AS
  SELECT c.id AS customer_id, c.name, c.country_code, c.tier,
         count(o.id)                         AS order_count,
         coalesce(sum(o.total), 0)           AS lifetime_value,
         max(o.ordered_at)                   AS last_order_at
  FROM customers c
  LEFT JOIN orders o ON o.customer_id = c.id AND o.status <> 'cancelled'
  GROUP BY c.id, c.name, c.country_code, c.tier;

CREATE VIEW top_products AS
  SELECT p.id, p.sku, p.name, p.category,
         sum(i.quantity) AS units_sold, sum(i.line_total) AS revenue
  FROM products p
  JOIN order_items i ON i.product_id = p.id
  GROUP BY p.id, p.sku, p.name, p.category;

ANALYZE;
"#;
const MYSQL_SCHEMA: &str = r#"
-- The entrypoint pipes this file through the mysql client on a connection whose
-- default charset is latin1, so without this every multi-byte name is stored
-- double-encoded: char_length('佐藤 彩') comes back as 10 and not 4.
SET NAMES utf8mb4;

SET FOREIGN_KEY_CHECKS = 0;
DROP VIEW  IF EXISTS top_products;
DROP VIEW  IF EXISTS customer_lifetime_value;
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS customers;
SET FOREIGN_KEY_CHECKS = 1;

CREATE TABLE customers (
  id            INT            NOT NULL PRIMARY KEY,
  name          VARCHAR(200)   NOT NULL,
  email         VARCHAR(200)   NOT NULL UNIQUE,
  country_code  CHAR(2)        NOT NULL,
  tier          ENUM('bronze','silver','gold','platinum') NOT NULL,
  credit_limit  DECIMAL(14,2)  NOT NULL,
  is_active     BOOLEAN        NOT NULL,
  signup_source VARCHAR(40)    NOT NULL,
  created_at    DATETIME       NOT NULL,
  metadata      JSON           NOT NULL,
  KEY idx_customers_tier (tier)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='Registered buyers.';

CREATE TABLE products (
  id          INT           NOT NULL PRIMARY KEY,
  sku         VARCHAR(32)   NOT NULL UNIQUE,
  name        VARCHAR(200)  NOT NULL,
  category    VARCHAR(60)   NOT NULL,
  price       DECIMAL(12,2) NOT NULL,
  weight_kg   DECIMAL(8,3)  NOT NULL,
  in_stock    BOOLEAN       NOT NULL,
  supplier    VARCHAR(120)  NOT NULL,
  released_on DATE          NOT NULL,
  tags        JSON          NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE orders (
  id          INT           NOT NULL PRIMARY KEY,
  customer_id INT           NOT NULL,
  status      ENUM('pending','processing','shipped','delivered','cancelled','refunded') NOT NULL,
  channel     VARCHAR(40)   NOT NULL,
  currency    CHAR(3)       NOT NULL,
  subtotal    DECIMAL(14,2) NOT NULL,
  tax         DECIMAL(14,2) NOT NULL,
  total       DECIMAL(14,2) NOT NULL,
  ordered_at  DATETIME      NOT NULL,
  shipped_at  DATETIME      NULL COMMENT 'NULL until the order ships.',
  note        VARCHAR(200)  NULL,
  KEY idx_orders_customer (customer_id),
  KEY idx_orders_ordered_at (ordered_at),
  CONSTRAINT fk_orders_customer FOREIGN KEY (customer_id) REFERENCES customers (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE order_items (
  id            INT           NOT NULL PRIMARY KEY,
  order_id      INT           NOT NULL,
  product_id    INT           NOT NULL,
  quantity      INT           NOT NULL,
  unit_price    DECIMAL(12,2) NOT NULL,
  discount_rate DECIMAL(4,2)  NOT NULL,
  line_total    DECIMAL(14,2) NOT NULL,
  gross_amount  DECIMAL(16,2) AS (unit_price * quantity) STORED,
  KEY idx_items_order (order_id),
  CONSTRAINT fk_items_order   FOREIGN KEY (order_id)   REFERENCES orders (id) ON DELETE CASCADE,
  CONSTRAINT fk_items_product FOREIGN KEY (product_id) REFERENCES products (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE events (
  id          INT          NOT NULL PRIMARY KEY,
  customer_id INT          NOT NULL,
  event_type  VARCHAR(40)  NOT NULL,
  occurred_at DATETIME     NOT NULL,
  session_id  VARCHAR(40)  NOT NULL,
  device      VARCHAR(40)  NOT NULL,
  duration_ms INT          NOT NULL,
  payload     JSON         NOT NULL,
  KEY idx_events_customer (customer_id, occurred_at),
  CONSTRAINT fk_events_customer FOREIGN KEY (customer_id) REFERENCES customers (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
"#;
const MYSQL_VIEWS: &str = r#"
CREATE VIEW customer_lifetime_value AS
  SELECT c.id AS customer_id, c.name, c.country_code, c.tier,
         COUNT(o.id) AS order_count, COALESCE(SUM(o.total), 0) AS lifetime_value,
         MAX(o.ordered_at) AS last_order_at
  FROM customers c
  LEFT JOIN orders o ON o.customer_id = c.id AND o.status <> 'cancelled'
  GROUP BY c.id, c.name, c.country_code, c.tier;

CREATE VIEW top_products AS
  SELECT p.id, p.sku, p.name, p.category,
         SUM(i.quantity) AS units_sold, SUM(i.line_total) AS revenue
  FROM products p
  JOIN order_items i ON i.product_id = p.id
  GROUP BY p.id, p.sku, p.name, p.category;
"#;
const ORACLE_SCHEMA: &str = r#"
WHENEVER SQLERROR CONTINUE;
DROP TABLE events CASCADE CONSTRAINTS PURGE;
DROP TABLE order_items CASCADE CONSTRAINTS PURGE;
DROP TABLE orders CASCADE CONSTRAINTS PURGE;
DROP TABLE products CASCADE CONSTRAINTS PURGE;
DROP TABLE customers CASCADE CONSTRAINTS PURGE;
WHENEVER SQLERROR EXIT SQL.SQLCODE;

CREATE TABLE customers (
  id            NUMBER(10)     PRIMARY KEY,
  name          VARCHAR2(200)  NOT NULL,
  email         VARCHAR2(200)  NOT NULL UNIQUE,
  country_code  CHAR(2)        NOT NULL,
  tier          VARCHAR2(10)   NOT NULL,
  credit_limit  NUMBER(14,2)   NOT NULL,
  is_active     NUMBER(1)      NOT NULL,
  signup_source VARCHAR2(40)   NOT NULL,
  created_at    TIMESTAMP      NOT NULL,
  metadata      VARCHAR2(4000) NOT NULL,
  CONSTRAINT ck_customers_meta CHECK (metadata IS JSON)
);

CREATE TABLE products (
  id          NUMBER(10)    PRIMARY KEY,
  sku         VARCHAR2(32)  NOT NULL UNIQUE,
  name        VARCHAR2(200) NOT NULL,
  category    VARCHAR2(60)  NOT NULL,
  price       NUMBER(12,2)  NOT NULL,
  weight_kg   NUMBER(8,3)   NOT NULL,
  in_stock    NUMBER(1)     NOT NULL,
  supplier    VARCHAR2(120) NOT NULL,
  released_on DATE          NOT NULL,
  tags        VARCHAR2(400) NOT NULL
);

CREATE TABLE orders (
  id          NUMBER(10)    PRIMARY KEY,
  customer_id NUMBER(10)    NOT NULL REFERENCES customers (id),
  status      VARCHAR2(12)  NOT NULL,
  channel     VARCHAR2(20)  NOT NULL,
  currency    CHAR(3)       NOT NULL,
  subtotal    NUMBER(14,2)  NOT NULL,
  tax         NUMBER(14,2)  NOT NULL,
  total       NUMBER(14,2)  NOT NULL,
  ordered_at  TIMESTAMP     NOT NULL,
  shipped_at  TIMESTAMP,
  note        VARCHAR2(200)
);

CREATE TABLE order_items (
  id            NUMBER(10)   PRIMARY KEY,
  order_id      NUMBER(10)   NOT NULL REFERENCES orders (id),
  product_id    NUMBER(10)   NOT NULL REFERENCES products (id),
  quantity      NUMBER(6)    NOT NULL,
  unit_price    NUMBER(12,2) NOT NULL,
  discount_rate NUMBER(4,2)  NOT NULL,
  line_total    NUMBER(14,2) NOT NULL,
  gross_amount  NUMBER(16,2) GENERATED ALWAYS AS (unit_price * quantity) VIRTUAL
);

CREATE TABLE events (
  id          NUMBER(10)     PRIMARY KEY,
  customer_id NUMBER(10)     NOT NULL REFERENCES customers (id),
  event_type  VARCHAR2(40)   NOT NULL,
  occurred_at TIMESTAMP      NOT NULL,
  session_id  VARCHAR2(40)   NOT NULL,
  device      VARCHAR2(40)   NOT NULL,
  duration_ms NUMBER(10)     NOT NULL,
  payload     VARCHAR2(4000) NOT NULL
);
"#;
const SQL_SERVER_SCHEMA: &str = r#"
IF DB_ID('samples') IS NULL EXEC('CREATE DATABASE samples');
GO
USE samples;
GO
SET QUOTED_IDENTIFIER ON;
SET ANSI_NULLS ON;
GO

DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS customers;
GO

CREATE TABLE customers (
  id            INT            NOT NULL PRIMARY KEY,
  name          NVARCHAR(200)  NOT NULL,
  email         NVARCHAR(200)  NOT NULL UNIQUE,
  country_code  CHAR(2)        NOT NULL,
  tier          VARCHAR(10)    NOT NULL,
  credit_limit  DECIMAL(14,2)  NOT NULL,
  is_active     BIT            NOT NULL,
  signup_source VARCHAR(40)    NOT NULL,
  created_at    DATETIME2(0)   NOT NULL,
  metadata      NVARCHAR(MAX)  NOT NULL
);

CREATE TABLE products (
  id          INT            NOT NULL PRIMARY KEY,
  sku         VARCHAR(32)    NOT NULL UNIQUE,
  name        NVARCHAR(200)  NOT NULL,
  category    VARCHAR(60)    NOT NULL,
  price       DECIMAL(12,2)  NOT NULL,
  weight_kg   DECIMAL(8,3)   NOT NULL,
  in_stock    BIT            NOT NULL,
  supplier    NVARCHAR(120)  NOT NULL,
  released_on DATE           NOT NULL,
  tags        NVARCHAR(MAX)  NOT NULL
);

CREATE TABLE orders (
  id          INT            NOT NULL PRIMARY KEY,
  customer_id INT            NOT NULL REFERENCES customers (id),
  status      VARCHAR(12)    NOT NULL,
  channel     VARCHAR(20)    NOT NULL,
  currency    CHAR(3)        NOT NULL,
  subtotal    DECIMAL(14,2)  NOT NULL,
  tax         DECIMAL(14,2)  NOT NULL,
  total       DECIMAL(14,2)  NOT NULL,
  ordered_at  DATETIME2(0)   NOT NULL,
  shipped_at  DATETIME2(0)   NULL,
  note        NVARCHAR(200)  NULL
);

CREATE TABLE order_items (
  id            INT           NOT NULL PRIMARY KEY,
  order_id      INT           NOT NULL REFERENCES orders (id),
  product_id    INT           NOT NULL REFERENCES products (id),
  quantity      INT           NOT NULL,
  unit_price    DECIMAL(12,2) NOT NULL,
  discount_rate DECIMAL(4,2)  NOT NULL,
  line_total    DECIMAL(14,2) NOT NULL,
  gross_amount  AS (unit_price * quantity) PERSISTED
);

CREATE TABLE events (
  id          INT           NOT NULL PRIMARY KEY,
  customer_id INT           NOT NULL REFERENCES customers (id),
  event_type  VARCHAR(40)   NOT NULL,
  occurred_at DATETIME2(0)  NOT NULL,
  session_id  VARCHAR(40)   NOT NULL,
  device      VARCHAR(40)   NOT NULL,
  duration_ms INT           NOT NULL,
  payload     NVARCHAR(MAX) NOT NULL
);
GO
"#;
const CLICKHOUSE_SCHEMA: &str = r#"
CREATE DATABASE IF NOT EXISTS samples;

DROP TABLE IF EXISTS samples.events;
DROP TABLE IF EXISTS samples.order_items;
DROP TABLE IF EXISTS samples.orders;
DROP TABLE IF EXISTS samples.products;
DROP TABLE IF EXISTS samples.customers;

CREATE TABLE samples.customers (
  id UInt32, name String, email String,
  country_code LowCardinality(String),
  tier Enum8('bronze' = 1, 'silver' = 2, 'gold' = 3, 'platinum' = 4),
  credit_limit Decimal(14, 2), is_active Bool,
  signup_source LowCardinality(String), created_at DateTime, metadata String
) ENGINE = MergeTree ORDER BY id;

CREATE TABLE samples.products (
  id UInt32, sku String, name String, category LowCardinality(String),
  price Decimal(12, 2), weight_kg Decimal(8, 3), in_stock Bool,
  supplier LowCardinality(String), released_on Date, tags String
) ENGINE = MergeTree ORDER BY id;

CREATE TABLE samples.orders (
  id UInt32, customer_id UInt32, status LowCardinality(String),
  channel LowCardinality(String), currency LowCardinality(String),
  subtotal Decimal(14, 2), tax Decimal(14, 2), total Decimal(14, 2),
  ordered_at DateTime, shipped_at Nullable(DateTime), note Nullable(String)
) ENGINE = MergeTree ORDER BY (ordered_at, id);

CREATE TABLE samples.order_items (
  id UInt32, order_id UInt32, product_id UInt32, quantity UInt16,
  unit_price Decimal(12, 2), discount_rate Decimal(4, 2), line_total Decimal(14, 2)
) ENGINE = MergeTree ORDER BY (order_id, id);

CREATE TABLE samples.events (
  id UInt32, customer_id UInt32, event_type LowCardinality(String),
  occurred_at DateTime, session_id String, device LowCardinality(String),
  duration_ms UInt32, payload String
) ENGINE = MergeTree ORDER BY (occurred_at, customer_id);
"#;
const SQLITE_SCHEMA: &str = r#"
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS customers;

CREATE TABLE customers (
  id INTEGER PRIMARY KEY, name VARCHAR NOT NULL, email VARCHAR NOT NULL UNIQUE,
  country_code VARCHAR NOT NULL, tier VARCHAR NOT NULL,
  credit_limit DECIMAL(14,2) NOT NULL, is_active BOOLEAN NOT NULL,
  signup_source VARCHAR NOT NULL, created_at TIMESTAMP NOT NULL, metadata VARCHAR NOT NULL
);
CREATE TABLE products (
  id INTEGER PRIMARY KEY, sku VARCHAR NOT NULL UNIQUE, name VARCHAR NOT NULL,
  category VARCHAR NOT NULL, price DECIMAL(12,2) NOT NULL, weight_kg DECIMAL(8,3) NOT NULL,
  in_stock BOOLEAN NOT NULL, supplier VARCHAR NOT NULL, released_on DATE NOT NULL, tags VARCHAR NOT NULL
);
CREATE TABLE orders (
  id INTEGER PRIMARY KEY, customer_id INTEGER NOT NULL REFERENCES customers (id),
  status VARCHAR NOT NULL, channel VARCHAR NOT NULL, currency VARCHAR NOT NULL,
  subtotal DECIMAL(14,2) NOT NULL, tax DECIMAL(14,2) NOT NULL, total DECIMAL(14,2) NOT NULL,
  ordered_at TIMESTAMP NOT NULL, shipped_at TIMESTAMP, note VARCHAR
);
CREATE TABLE order_items (
  id INTEGER PRIMARY KEY, order_id INTEGER NOT NULL REFERENCES orders (id),
  product_id INTEGER NOT NULL REFERENCES products (id), quantity INTEGER NOT NULL,
  unit_price DECIMAL(12,2) NOT NULL, discount_rate DECIMAL(4,2) NOT NULL, line_total DECIMAL(14,2) NOT NULL
);
CREATE TABLE events (
  id INTEGER PRIMARY KEY, customer_id INTEGER NOT NULL REFERENCES customers (id),
  event_type VARCHAR NOT NULL, occurred_at TIMESTAMP NOT NULL, session_id VARCHAR NOT NULL,
  device VARCHAR NOT NULL, duration_ms INTEGER NOT NULL, payload VARCHAR NOT NULL
);
"#;
