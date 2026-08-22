-- Cassandra feature sample for Irodori Table.
-- Run against `task start -- cassandra`.
--
-- CQL has no joins. The model is a table per access pattern: `orders` and
-- `orders_by_customer` hold the same rows under different primary keys.

DESCRIBE KEYSPACE samples;

SELECT release_version FROM system.local;

-- Lookup by partition key.
SELECT * FROM samples.orders WHERE id = 1;

-- The denormalised table: this customer's orders, newest first, one partition.
SELECT id, ordered_at, status, total
FROM samples.orders_by_customer
WHERE customer_id = 42;

-- Clustered read: the lines under one order.
SELECT * FROM samples.order_items WHERE order_id = 1;

-- Clickstream clustered under the customer.
SELECT occurred_at, event_type, device, duration_ms
FROM samples.events
WHERE customer_id = 42
LIMIT 20;

-- A secondary index is a different animal here than in a row store.
SELECT id, name, tier FROM samples.customers WHERE tier = 'platinum' LIMIT 20;
