-- ScyllaDB feature sample for Irodori Table.
-- Run against `task start -- scylladb`.
--
-- Same CQL schema and queries as the Cassandra sample; Irodori routes both
-- through one adapter, so this is where the two should agree.

DESCRIBE KEYSPACE samples;

SELECT * FROM samples.orders WHERE id = 1;

SELECT id, ordered_at, status, total
FROM samples.orders_by_customer
WHERE customer_id = 42;

SELECT * FROM samples.order_items WHERE order_id = 1;

SELECT occurred_at, event_type, device, duration_ms
FROM samples.events
WHERE customer_id = 42
LIMIT 20;
