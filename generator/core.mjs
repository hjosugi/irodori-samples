// Deterministic sample dataset, shared by every emitter in this directory.
//
// Same generator as the irodori-table database playground, dialled down: these
// seeds are committed to the repo and loaded by container init hooks, so they
// have to stay small enough to read and to boot quickly. Nothing here reads the
// clock or Math.random(), so regenerating produces byte-identical files.
//
//   SCALE=1 node generator/generate.mjs   # the playground's full size
const SCALE = Number(process.env.SCALE ?? 0.02)
const SEED = Number(process.env.SEED ?? 20260807)

export const N = {
  customers: Math.max(1, Math.round(10_000 * SCALE)),
  products: Math.max(1, Math.round(2_000 * SCALE)),
  orders: Math.max(1, Math.round(30_000 * SCALE)),
  events: Math.max(1, Math.round(50_000 * SCALE)),
}

// --- deterministic PRNG (mulberry32) ----------------------------------------
let state = SEED >>> 0
const rnd = () => {
  state = (state + 0x6d2b79f5) >>> 0
  let t = state
  t = Math.imul(t ^ (t >>> 15), t | 1)
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296
}
const int = (min, max) => min + Math.floor(rnd() * (max - min + 1))
const pick = (arr) => arr[Math.floor(rnd() * arr.length)]
const money = (min, max) => (min + rnd() * (max - min)).toFixed(2)
// Weighted pick: [[value, weight], ...]. Keeps status/tier distributions skewed
// like real data, so GROUP BY results are interesting rather than uniform.
const weighted = (pairs) => {
  const total = pairs.reduce((s, [, w]) => s + w, 0)
  let r = rnd() * total
  for (const [v, w] of pairs) {
    if ((r -= w) < 0) return v
  }
  return pairs[pairs.length - 1][0]
}

// --- fixed time window ------------------------------------------------------
// No Date.now(): the dataset must not drift between runs.
const T0 = Date.parse('2024-01-01T00:00:00Z')
const T1 = Date.parse('2026-08-01T00:00:00Z')
const SPAN = T1 - T0
const pad = (n) => String(n).padStart(2, '0')
// "YYYY-MM-DD HH:MM:SS" is the one timestamp spelling every loader here accepts
// without a per-engine format hint.
const ts = (ms) => {
  const d = new Date(ms)
  return (
    `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ` +
    `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`
  )
}
const dateOnly = (ms) => ts(ms).slice(0, 10)
// The CSV spelling has no sub-second part, so BSON dates have to be truncated
// to match. Otherwise the same logical row is a different instant in MongoDB
// than it is in every engine loaded from the CSV.
const isoTs = (ms) => new Date(Math.floor(ms / 1000) * 1000).toISOString()

// --- vocabulary -------------------------------------------------------------
const GIVEN = ['Aoi', 'Haruto', 'Mei', 'Ren', 'Yuna', 'Sora', 'Kaito', 'Rin', 'Hana', 'Riku',
  'Emma', 'Liam', 'Olivia', 'Noah', 'Ava', 'Ethan', 'Mia', 'Lucas', 'Chloe', 'Leo',
  'Sofia', 'Mateo', 'Lena', 'Jonas', 'Elif', 'Omar', 'Ines', 'Nikolai', 'Sanne', 'Tomas']
const FAMILY = ['Sato', 'Suzuki', 'Takahashi', 'Tanaka', 'Watanabe', 'Ito', 'Nakamura', 'Kobayashi',
  'Smith', 'Johnson', 'Brown', 'Garcia', 'Muller', 'Rossi', 'Dubois', 'Novak',
  'Silva', 'Kim', 'Chen', 'Nguyen', 'Andersson', 'Okafor', 'Haddad', 'Petrov']
// A deliberate slice of non-ASCII, RTL and emoji names. A table viewer has to
// get column widths, alignment and bidi runs right; ASCII-only data hides that.
const UNICODE_NAMES = ['佐藤 彩', '田中 陽翔', '山本 美咲', '株式会社 彩テーブル', '김민준', '이서연',
  '张伟', '王芳', 'محمد الأحمد', 'فاطمة الزهراء', 'Ελένη Παπαδοπούλου', 'Йосип Ковальчук',
  'Zoë Müller‑Groß', 'Renée Dubois‑Lefèvre', '🌸 Sakura Trading', '⚡ Volt Industries',
  'Ñandú Logística', 'Þórunn Jónsdóttir', 'Đặng Thị Hương', 'ราชวงศ์ จักรี']
const COUNTRIES = [['JP', 12], ['US', 10], ['GB', 5], ['DE', 5], ['FR', 4], ['KR', 4], ['TW', 3],
  ['SG', 3], ['AU', 3], ['BR', 3], ['IN', 4], ['CA', 3], ['NL', 2], ['SE', 2], ['ES', 2], ['IT', 2]]
const TIERS = [['bronze', 50], ['silver', 30], ['gold', 15], ['platinum', 5]]
const SOURCES = ['organic', 'referral', 'paid_search', 'social', 'partner', 'event']
const CATEGORIES = ['Kitchen', 'Outdoor', 'Office', 'Audio', 'Lighting', 'Storage',
  'Textiles', 'Stationery', 'Tools', 'Wellness']
const ADJ = ['Compact', 'Nordic', 'Matte', 'Bamboo', 'Ceramic', 'Linen', 'Copper', 'Walnut',
  'Slate', 'Ultra', 'Folding', 'Insulated', 'Modular', 'Woven', 'Brushed']
const NOUN = ['Kettle', 'Lantern', 'Desk Mat', 'Speaker', 'Sconce', 'Crate', 'Throw', 'Notebook',
  'Wrench', 'Diffuser', 'Tumbler', 'Shelf', 'Cushion', 'Planter', 'Stool']
const SUPPLIERS = ['Kawase Foods', 'Northwind Retail', 'Aster Works', 'Minato Labs',
  'Brightfold Supply', 'Terrace & Co', 'Hokuto Manufacturing', 'Lumen Partners']
const ORDER_STATUS = [['delivered', 45], ['shipped', 20], ['processing', 15], ['pending', 10],
  ['cancelled', 6], ['refunded', 4]]
const CURRENCIES = [['JPY', 40], ['USD', 30], ['EUR', 15], ['GBP', 8], ['AUD', 4], ['SGD', 3]]
const CHANNELS = [['web', 55], ['mobile_app', 30], ['marketplace', 10], ['phone', 5]]
const EVENT_TYPES = [['page_view', 40], ['add_to_cart', 20], ['search', 15], ['checkout_start', 10],
  ['purchase', 8], ['support_ticket', 4], ['refund_request', 3]]
const DEVICES = [['ios', 30], ['android', 28], ['desktop_chrome', 25], ['desktop_safari', 10], ['desktop_firefox', 7]]
const TAG_POOL = ['bestseller', 'new', 'clearance', 'eco', 'fragile', 'bulky', 'gift', 'limited', 'refill']


// --- dataset ----------------------------------------------------------------
// Built once, in memory, and handed to every emitter. The relationships hold:
// an order never predates its customer, and orders.subtotal is the exact sum of
// its own lines, so aggregate queries reconcile on any engine.
export function build() {
  const customers = []
  const products = []
  const orders = []
  const orderItems = []
  const events = []

  for (let id = 1; id <= N.customers; id++) {
    const name = id % 40 === 7
      ? UNICODE_NAMES[Math.floor(id / 40) % UNICODE_NAMES.length]
      : `${pick(GIVEN)} ${pick(FAMILY)}`
    const country = weighted(COUNTRIES)
    const tier = weighted(TIERS)
    const created = T0 + Math.floor(rnd() * SPAN * 0.8)
    customers.push({
      id,
      name,
      email: `customer${id}@example.com`,
      country_code: country,
      tier,
      credit_limit: Number(money(0, tier === 'platinum' ? 5_000_000 : tier === 'gold' ? 1_000_000 : 200_000)),
      is_active: rnd() > 0.12,
      signup_source: pick(SOURCES),
      created_at: ts(created),
      created_ms: created,
      metadata: {
        segment: tier === 'platinum' || tier === 'gold' ? 'enterprise' : 'smb',
        churn_risk: Number(rnd().toFixed(3)),
        newsletter: rnd() > 0.4,
        locale: country === 'JP' ? 'ja-JP' : country === 'KR' ? 'ko-KR' : 'en-US',
      },
    })
  }

  for (let id = 1; id <= N.products; id++) {
    products.push({
      id,
      sku: `SKU-${String(id).padStart(6, '0')}`,
      name: `${pick(ADJ)} ${pick(NOUN)}`,
      category: pick(CATEGORIES),
      price: Number(money(300, 90_000)),
      weight_kg: Number((0.05 + rnd() * 20).toFixed(3)),
      in_stock: rnd() > 0.18,
      supplier: pick(SUPPLIERS),
      released_on: dateOnly(T0 - 365 * 86400_000 + Math.floor(rnd() * SPAN)),
      tags: Array.from(new Set([pick(TAG_POOL), pick(TAG_POOL)])),
    })
  }

  let itemId = 0
  for (let id = 1; id <= N.orders; id++) {
    const customer = customers[int(1, N.customers) - 1]
    const status = weighted(ORDER_STATUS)
    const orderedAt = customer.created_ms +
      Math.floor(rnd() * Math.max(1, T1 - customer.created_ms))
    const lines = []
    let subtotal = 0
    const lineCount = int(1, 6)
    for (let i = 0; i < lineCount; i++) {
      const product = products[int(1, N.products) - 1]
      const qty = weighted([[1, 50], [2, 25], [3, 12], [4, 6], [5, 4], [8, 2], [12, 1]])
      const discount = weighted([[0, 60], [0.05, 15], [0.1, 12], [0.15, 8], [0.25, 5]])
      const lineTotal = Number((product.price * qty * (1 - discount)).toFixed(2))
      subtotal += lineTotal
      itemId++
      const line = {
        id: itemId,
        order_id: id,
        product_id: product.id,
        quantity: qty,
        unit_price: product.price,
        discount_rate: discount,
        line_total: lineTotal,
      }
      lines.push(line)
      orderItems.push(line)
    }
    subtotal = Number(subtotal.toFixed(2))
    const tax = Number((subtotal * 0.1).toFixed(2))
    orders.push({
      id,
      customer_id: customer.id,
      status,
      channel: weighted(CHANNELS),
      currency: weighted(CURRENCIES),
      subtotal,
      tax,
      total: Number((subtotal + tax).toFixed(2)),
      ordered_at: ts(orderedAt),
      ordered_ms: orderedAt,
      shipped_at: status === 'delivered' || status === 'shipped' || status === 'refunded'
        ? ts(orderedAt + int(3600, 7 * 86400) * 1000)
        : null,
      note: rnd() > 0.9
        ? pick(['ギフト包装希望', 'leave at door', '午前中指定', 'fragile — handle with care', '納品書同梱'])
        : null,
      items: lines,
    })
  }

  for (let id = 1; id <= N.events; id++) {
    const customer = customers[int(1, N.customers) - 1]
    const occurredAt = customer.created_ms +
      Math.floor(rnd() * Math.max(1, T1 - customer.created_ms))
    events.push({
      id,
      customer_id: customer.id,
      event_type: weighted(EVENT_TYPES),
      occurred_at: ts(occurredAt),
      occurred_ms: occurredAt,
      session_id: `sess-${(id * 2654435761 % 4294967296).toString(16).padStart(8, '0')}`,
      device: weighted(DEVICES),
      duration_ms: int(15, 90_000),
      payload: {
        path: pick(['/', '/search', '/cart', '/checkout', '/product', '/account', '/support']),
        ab_bucket: pick(['control', 'variant_a', 'variant_b']),
        value: Number((rnd() * 1000).toFixed(2)),
      },
    })
  }

  return { customers, products, orders, orderItems, events }
}

export const meta = { SCALE, SEED }
