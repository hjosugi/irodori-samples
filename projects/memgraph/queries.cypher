// Memgraph feature sample for Irodori Table.
// Run against `task start -- memgraph`.
// Needs the `irodori.memgraph` connector extension.
//
// Same graph as the Neo4j sample; the dialects differ mainly in index syntax.

MATCH (n) RETURN Labels(n)[0] AS label, Count(*) AS nodes ORDER BY label;
MATCH ()-[r]->() RETURN Type(r) AS rel, Count(*) AS rels ORDER BY rel;

MATCH path = (c:Customer)-[:PLACED]->(o:Order)-[:CONTAINS]->(p:Product)
WHERE c.tier = "platinum"
RETURN path LIMIT 50;

MATCH (c:Customer)-[:PLACED]->(o:Order)
WHERE NOT o.status IN ["cancelled", "refunded"]
RETURN c.id, c.name, Count(o) AS orders, Round(Sum(o.total)) AS lifetime_value
ORDER BY lifetime_value DESC LIMIT 20;

SHOW INDEX INFO;
SHOW CONSTRAINT INFO;
