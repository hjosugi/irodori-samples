-- InfluxDB 3 feature sample for Irodori Table.
-- Run against `task up -- influxdb`.
--
-- Only the two time-series tables map onto a measurement. A point is keyed by
-- measurement + tags + timestamp, so rows sharing all three collapse into one:
-- the counts here can be lower than the source table by design.

SELECT status, count(*) AS orders, sum(total) AS revenue
FROM orders
GROUP BY status
ORDER BY revenue DESC;

SELECT event_type, device, count(*) AS events, avg(duration_ms) AS avg_ms
FROM events
GROUP BY event_type, device
ORDER BY events DESC;

SELECT date_bin(INTERVAL '1 day', time) AS day, count(*) AS events
FROM events
GROUP BY day
ORDER BY day DESC
LIMIT 30;
