// Neo4j feature sample for Irodori Table.
// Run against `task start -- neo4j`.
//
// (:Customer)-[:PLACED]->(:Order)-[:CONTAINS]->(:Product)
// (:Customer)-[:TRIGGERED]->(:Event)

MATCH (n) RETURN labels(n)[0] AS label, count(*) AS nodes ORDER BY label;
MATCH ()-[r]->() RETURN type(r) AS rel, count(*) AS rels ORDER BY rel;

// A subgraph, for the graph view.
MATCH path = (c:Customer)-[:PLACED]->(o:Order)-[:CONTAINS]->(p:Product)
WHERE c.tier = 'platinum'
RETURN path LIMIT 50;

// Aggregate over a traversal.
MATCH (c:Customer)-[:PLACED]->(o:Order)
WHERE NOT o.status IN ['cancelled', 'refunded']
RETURN c.id, c.name, count(o) AS orders, round(sum(o.total)) AS lifetime_value
ORDER BY lifetime_value DESC LIMIT 20;

// Co-purchase: the query shape that is painful in SQL and natural here.
MATCH (p:Product {id: 7})<-[:CONTAINS]-(:Order)-[:CONTAINS]->(other:Product)
WHERE other.id <> p.id
RETURN other.sku, other.name, count(*) AS bought_together
ORDER BY bought_together DESC LIMIT 10;

CALL db.schema.visualization();
SHOW INDEXES;
