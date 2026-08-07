-- QuestDB feature sample for Irodori Table.
-- Run against `task up -- questdb`.

SELECT build();

-- SAMPLE BY: interval aggregation with no GROUP BY.
SELECT ordered_at, currency, count() AS orders, sum(total) AS revenue
FROM orders
WHERE status NOT IN ('cancelled', 'refunded')
SAMPLE BY 1M
ORDER BY ordered_at;

-- LATEST ON: the most recent row per key, in one pass.
SELECT * FROM events
LATEST ON occurred_at
PARTITION BY customer_id
LIMIT 20;

-- ASOF JOIN: join each event to the order in effect at that moment. The
-- QuestDB feature with no plain-SQL equivalent.
SELECT e.occurred_at, e.customer_id, e.event_type, o.id AS last_order, o.total
FROM events e
ASOF JOIN orders o ON (customer_id)
LIMIT 50;

-- SYMBOL columns are dictionary-encoded; these are the cheap ones to group on.
SHOW COLUMNS FROM events;
