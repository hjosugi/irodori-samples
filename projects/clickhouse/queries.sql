-- ClickHouse feature sample for Irodori Table.
-- Run against `task up -- clickhouse`.

SELECT version();

-- Column types a row store has no equivalent for.
DESCRIBE TABLE samples.customers;

-- Approximate distinct counts and quantiles: cheap here, expensive elsewhere.
SELECT event_type,
       count()                           AS events,
       uniq(customer_id)                 AS approx_customers,
       round(quantile(0.95)(duration_ms)) AS p95_ms
FROM samples.events
GROUP BY event_type
ORDER BY events DESC;

-- JSON kept as String, parsed at query time.
SELECT JSONExtractString(metadata, 'segment') AS segment,
       count() AS customers
FROM samples.customers
GROUP BY segment;

-- Partitions and parts as they sit on disk.
SELECT table, partition, sum(rows) AS rows
FROM system.parts
WHERE database = 'samples' AND active
GROUP BY table, partition
ORDER BY table, partition;
