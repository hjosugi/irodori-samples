const T0: i64 = 1_704_067_200_000;
const T1: i64 = 1_785_542_400_000;
const SPAN: i64 = T1 - T0;

const GIVEN: &[&str] = &[
    "Aoi", "Haruto", "Mei", "Ren", "Yuna", "Sora", "Kaito", "Rin", "Hana", "Riku", "Emma", "Liam",
    "Olivia", "Noah", "Ava", "Ethan", "Mia", "Lucas", "Chloe", "Leo", "Sofia", "Mateo", "Lena",
    "Jonas", "Elif", "Omar", "Ines", "Nikolai", "Sanne", "Tomas",
];
const FAMILY: &[&str] = &[
    "Sato",
    "Suzuki",
    "Takahashi",
    "Tanaka",
    "Watanabe",
    "Ito",
    "Nakamura",
    "Kobayashi",
    "Smith",
    "Johnson",
    "Brown",
    "Garcia",
    "Muller",
    "Rossi",
    "Dubois",
    "Novak",
    "Silva",
    "Kim",
    "Chen",
    "Nguyen",
    "Andersson",
    "Okafor",
    "Haddad",
    "Petrov",
];
const UNICODE_NAMES: &[&str] = &[
    "佐藤 彩",
    "田中 陽翔",
    "山本 美咲",
    "株式会社 彩テーブル",
    "김민준",
    "이서연",
    "张伟",
    "王芳",
    "محمد الأحمد",
    "فاطمة الزهراء",
    "Ελένη Παπαδοπούλου",
    "Йосип Ковальчук",
    "Zoë Müller‑Groß",
    "Renée Dubois‑Lefèvre",
    "🌸 Sakura Trading",
    "⚡ Volt Industries",
    "Ñandú Logística",
    "Þórunn Jónsdóttir",
    "Đặng Thị Hương",
    "ราชวงศ์ จักรี",
];
const COUNTRIES: &[(&str, u32)] = &[
    ("JP", 12),
    ("US", 10),
    ("GB", 5),
    ("DE", 5),
    ("FR", 4),
    ("KR", 4),
    ("TW", 3),
    ("SG", 3),
    ("AU", 3),
    ("BR", 3),
    ("IN", 4),
    ("CA", 3),
    ("NL", 2),
    ("SE", 2),
    ("ES", 2),
    ("IT", 2),
];
const TIERS: &[(&str, u32)] = &[
    ("bronze", 50),
    ("silver", 30),
    ("gold", 15),
    ("platinum", 5),
];
const SOURCES: &[&str] = &[
    "organic",
    "referral",
    "paid_search",
    "social",
    "partner",
    "event",
];
const CATEGORIES: &[&str] = &[
    "Kitchen",
    "Outdoor",
    "Office",
    "Audio",
    "Lighting",
    "Storage",
    "Textiles",
    "Stationery",
    "Tools",
    "Wellness",
];
const ADJECTIVES: &[&str] = &[
    "Compact",
    "Nordic",
    "Matte",
    "Bamboo",
    "Ceramic",
    "Linen",
    "Copper",
    "Walnut",
    "Slate",
    "Ultra",
    "Folding",
    "Insulated",
    "Modular",
    "Woven",
    "Brushed",
];
const NOUNS: &[&str] = &[
    "Kettle", "Lantern", "Desk Mat", "Speaker", "Sconce", "Crate", "Throw", "Notebook", "Wrench",
    "Diffuser", "Tumbler", "Shelf", "Cushion", "Planter", "Stool",
];
const SUPPLIERS: &[&str] = &[
    "Kawase Foods",
    "Northwind Retail",
    "Aster Works",
    "Minato Labs",
    "Brightfold Supply",
    "Terrace & Co",
    "Hokuto Manufacturing",
    "Lumen Partners",
];
const ORDER_STATUSES: &[(&str, u32)] = &[
    ("delivered", 45),
    ("shipped", 20),
    ("processing", 15),
    ("pending", 10),
    ("cancelled", 6),
    ("refunded", 4),
];
const CURRENCIES: &[(&str, u32)] = &[
    ("JPY", 40),
    ("USD", 30),
    ("EUR", 15),
    ("GBP", 8),
    ("AUD", 4),
    ("SGD", 3),
];
const CHANNELS: &[(&str, u32)] = &[
    ("web", 55),
    ("mobile_app", 30),
    ("marketplace", 10),
    ("phone", 5),
];
const EVENT_TYPES: &[(&str, u32)] = &[
    ("page_view", 40),
    ("add_to_cart", 20),
    ("search", 15),
    ("checkout_start", 10),
    ("purchase", 8),
    ("support_ticket", 4),
    ("refund_request", 3),
];
const DEVICES: &[(&str, u32)] = &[
    ("ios", 30),
    ("android", 28),
    ("desktop_chrome", 25),
    ("desktop_safari", 10),
    ("desktop_firefox", 7),
];
const TAGS: &[&str] = &[
    "bestseller",
    "new",
    "clearance",
    "eco",
    "fragile",
    "bulky",
    "gift",
    "limited",
    "refill",
];
const NOTES: &[&str] = &[
    "ギフト包装希望",
    "leave at door",
    "午前中指定",
    "fragile — handle with care",
    "納品書同梱",
];
const PATHS: &[&str] = &[
    "/",
    "/search",
    "/cart",
    "/checkout",
    "/product",
    "/account",
    "/support",
];
const AB_BUCKETS: &[&str] = &["control", "variant_a", "variant_b"];
const QUANTITIES: &[(u32, u32)] = &[(1, 50), (2, 25), (3, 12), (4, 6), (5, 4), (8, 2), (12, 1)];
const DISCOUNTS: &[(f64, u32)] = &[(0.0, 60), (0.05, 15), (0.1, 12), (0.15, 8), (0.25, 5)];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Counts {
    pub customers: usize,
    pub products: usize,
    pub orders: usize,
    pub events: usize,
}

impl Counts {
    pub fn from_scale(scale: f64) -> Self {
        Self {
            customers: scaled(10_000, scale),
            products: scaled(2_000, scale),
            orders: scaled(30_000, scale),
            events: scaled(50_000, scale),
        }
    }
}

#[derive(Debug)]
pub(super) struct Metadata {
    pub segment: &'static str,
    pub churn_risk: f64,
    pub newsletter: bool,
    pub locale: &'static str,
}

#[derive(Debug)]
pub(super) struct Customer {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub country_code: &'static str,
    pub tier: &'static str,
    pub credit_limit: f64,
    pub is_active: bool,
    pub signup_source: &'static str,
    pub created_at: String,
    pub created_ms: i64,
    pub metadata: Metadata,
}

#[derive(Debug)]
pub(super) struct Product {
    pub id: u32,
    pub sku: String,
    pub name: String,
    pub category: &'static str,
    pub price: f64,
    pub weight_kg: f64,
    pub in_stock: bool,
    pub supplier: &'static str,
    pub released_on: String,
    pub tags: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub(super) struct OrderItem {
    pub id: u32,
    pub order_id: u32,
    pub product_id: u32,
    pub quantity: u32,
    pub unit_price: f64,
    pub discount_rate: f64,
    pub line_total: f64,
}

#[derive(Debug)]
pub(super) struct Order {
    pub id: u32,
    pub customer_id: u32,
    pub status: &'static str,
    pub channel: &'static str,
    pub currency: &'static str,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub ordered_at: String,
    pub ordered_ms: i64,
    pub shipped_at: Option<String>,
    pub note: Option<&'static str>,
    pub items: Vec<OrderItem>,
}

#[derive(Debug)]
pub(super) struct Payload {
    pub path: &'static str,
    pub ab_bucket: &'static str,
    pub value: f64,
}

#[derive(Debug)]
pub(super) struct Event {
    pub id: u32,
    pub customer_id: u32,
    pub event_type: &'static str,
    pub occurred_at: String,
    pub occurred_ms: i64,
    pub session_id: String,
    pub device: &'static str,
    pub duration_ms: u32,
    pub payload: Payload,
}

#[derive(Debug)]
pub(super) struct Dataset {
    pub customers: Vec<Customer>,
    pub products: Vec<Product>,
    pub orders: Vec<Order>,
    pub order_items: Vec<OrderItem>,
    pub events: Vec<Event>,
}

pub(super) fn build(counts: Counts, seed: u32) -> Dataset {
    let mut random = Mulberry32::new(seed);
    let mut customers = Vec::with_capacity(counts.customers);
    let mut products = Vec::with_capacity(counts.products);
    let mut orders = Vec::with_capacity(counts.orders);
    let mut order_items = Vec::with_capacity(counts.orders * 4);
    let mut events = Vec::with_capacity(counts.events);

    for id in 1..=counts.customers as u32 {
        let name = if id % 40 == 7 {
            UNICODE_NAMES[(id as usize / 40) % UNICODE_NAMES.len()].to_owned()
        } else {
            format!("{} {}", random.pick(GIVEN), random.pick(FAMILY))
        };
        let country = random.weighted(COUNTRIES);
        let tier = random.weighted(TIERS);
        let created_ms = T0 + (random.next() * SPAN as f64 * 0.8).floor() as i64;
        let maximum_credit = if tier == "platinum" {
            5_000_000.0
        } else if tier == "gold" {
            1_000_000.0
        } else {
            200_000.0
        };
        customers.push(Customer {
            id,
            name,
            email: format!("customer{id}@example.com"),
            country_code: country,
            tier,
            credit_limit: fixed(random.next() * maximum_credit, 2),
            is_active: random.next() > 0.12,
            signup_source: random.pick(SOURCES),
            created_at: timestamp(created_ms),
            created_ms,
            metadata: Metadata {
                segment: if matches!(tier, "platinum" | "gold") {
                    "enterprise"
                } else {
                    "smb"
                },
                churn_risk: fixed(random.next(), 3),
                newsletter: random.next() > 0.4,
                locale: match country {
                    "JP" => "ja-JP",
                    "KR" => "ko-KR",
                    _ => "en-US",
                },
            },
        });
    }

    for id in 1..=counts.products as u32 {
        let name = format!("{} {}", random.pick(ADJECTIVES), random.pick(NOUNS));
        let category = random.pick(CATEGORIES);
        let price = fixed(300.0 + random.next() * (90_000.0 - 300.0), 2);
        let weight_kg = fixed(0.05 + random.next() * 20.0, 3);
        let in_stock = random.next() > 0.18;
        let supplier = random.pick(SUPPLIERS);
        let released_on =
            date_only(T0 - 365 * 86_400_000 + (random.next() * SPAN as f64).floor() as i64);
        let first_tag = random.pick(TAGS);
        let second_tag = random.pick(TAGS);
        let tags = if first_tag == second_tag {
            vec![first_tag]
        } else {
            vec![first_tag, second_tag]
        };
        products.push(Product {
            id,
            sku: format!("SKU-{id:06}"),
            name,
            category,
            price,
            weight_kg,
            in_stock,
            supplier,
            released_on,
            tags,
        });
    }

    let mut item_id = 0;
    for id in 1..=counts.orders as u32 {
        let customer_index = random.integer(1, counts.customers as u32) as usize - 1;
        let customer = &customers[customer_index];
        let status = random.weighted(ORDER_STATUSES);
        let remaining = (T1 - customer.created_ms).max(1);
        let ordered_ms = customer.created_ms + (random.next() * remaining as f64).floor() as i64;
        let line_count = random.integer(1, 6);
        let mut items = Vec::with_capacity(line_count as usize);
        let mut subtotal = 0.0;
        for _ in 0..line_count {
            let product_index = random.integer(1, counts.products as u32) as usize - 1;
            let product = &products[product_index];
            let quantity = random.weighted(QUANTITIES);
            let discount_rate = random.weighted(DISCOUNTS);
            let line_total = fixed(product.price * quantity as f64 * (1.0 - discount_rate), 2);
            subtotal += line_total;
            item_id += 1;
            let item = OrderItem {
                id: item_id,
                order_id: id,
                product_id: product.id,
                quantity,
                unit_price: product.price,
                discount_rate,
                line_total,
            };
            items.push(item.clone());
            order_items.push(item);
        }
        let subtotal = fixed(subtotal, 2);
        let tax = fixed(subtotal * 0.1, 2);
        let total = fixed(subtotal + tax, 2);
        let channel = random.weighted(CHANNELS);
        let currency = random.weighted(CURRENCIES);
        let shipped_at = matches!(status, "delivered" | "shipped" | "refunded")
            .then(|| timestamp(ordered_ms + random.integer(3_600, 7 * 86_400) as i64 * 1_000));
        let note = (random.next() > 0.9).then(|| random.pick(NOTES));
        orders.push(Order {
            id,
            customer_id: customer.id,
            status,
            channel,
            currency,
            subtotal,
            tax,
            total,
            ordered_at: timestamp(ordered_ms),
            ordered_ms,
            shipped_at,
            note,
            items,
        });
    }

    for id in 1..=counts.events as u32 {
        let customer_index = random.integer(1, counts.customers as u32) as usize - 1;
        let customer = &customers[customer_index];
        let remaining = (T1 - customer.created_ms).max(1);
        let occurred_ms = customer.created_ms + (random.next() * remaining as f64).floor() as i64;
        events.push(Event {
            id,
            customer_id: customer.id,
            event_type: random.weighted(EVENT_TYPES),
            occurred_at: timestamp(occurred_ms),
            occurred_ms,
            session_id: format!("sess-{:08x}", id.wrapping_mul(2_654_435_761)),
            device: random.weighted(DEVICES),
            duration_ms: random.integer(15, 90_000),
            payload: Payload {
                path: random.pick(PATHS),
                ab_bucket: random.pick(AB_BUCKETS),
                value: fixed(random.next() * 1_000.0, 2),
            },
        });
    }

    Dataset {
        customers,
        products,
        orders,
        order_items,
        events,
    }
}

fn scaled(base: usize, scale: f64) -> usize {
    ((base as f64 * scale).round() as usize).max(1)
}

fn fixed(value: f64, digits: usize) -> f64 {
    debug_assert!(value.is_finite());
    let negative = value.is_sign_negative();
    let bits = value.abs().to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, binary_exponent) = if exponent_bits == 0 {
        (fraction, 1 - 1023 - 52)
    } else {
        (fraction | (1_u64 << 52), exponent_bits - 1023 - 52)
    };
    let decimal_factor = 10_u128.pow(digits as u32);
    let scaled = u128::from(significand) * decimal_factor;
    let rounded = if binary_exponent >= 0 {
        scaled << binary_exponent
    } else {
        let shift = (-binary_exponent) as u32;
        if shift >= 128 {
            0
        } else {
            let whole = scaled >> shift;
            let remainder = scaled - (whole << shift);
            let halfway = 1_u128 << (shift - 1);
            whole + u128::from(remainder >= halfway)
        }
    };

    let mut decimal = rounded.to_string();
    if digits > 0 {
        if decimal.len() <= digits {
            decimal.insert_str(0, &"0".repeat(digits + 1 - decimal.len()));
        }
        decimal.insert(decimal.len() - digits, '.');
    }
    let rounded = decimal
        .parse::<f64>()
        .expect("a fixed-point sample number remains finite");
    if negative { -rounded } else { rounded }
}

fn timestamp(milliseconds: i64) -> String {
    let seconds = milliseconds.div_euclid(1_000);
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = seconds_of_day % 3_600 / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

pub(super) fn iso_timestamp(milliseconds: i64) -> String {
    timestamp(milliseconds).replace(' ', "T") + ".000Z"
}

fn date_only(milliseconds: i64) -> String {
    timestamp(milliseconds)[..10].to_owned()
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    // Howard Hinnant's proleptic-Gregorian conversion, with Unix epoch offset.
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut value = self.state;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
        (value ^ (value >> 14)) as f64 / 4_294_967_296.0
    }

    fn integer(&mut self, minimum: u32, maximum: u32) -> u32 {
        minimum + (self.next() * f64::from(maximum - minimum + 1)).floor() as u32
    }

    fn pick<T: Copy>(&mut self, values: &[T]) -> T {
        values[(self.next() * values.len() as f64).floor() as usize]
    }

    fn weighted<T: Copy>(&mut self, values: &[(T, u32)]) -> T {
        let total = values.iter().map(|(_, weight)| *weight).sum::<u32>();
        let mut remaining = self.next() * f64::from(total);
        for (value, weight) in values {
            remaining -= f64::from(*weight);
            if remaining < 0.0 {
                return *value;
            }
        }
        values.last().expect("weighted values are non-empty").0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_matches_the_original_sequence() {
        let mut random = Mulberry32::new(20_260_807);
        let values = [random.next(), random.next(), random.next()];
        assert_eq!(
            values,
            [
                0.379_795_835_353_434_1,
                0.175_841_050_921_008,
                0.759_533_906_355_500_2,
            ]
        );
    }

    #[test]
    fn timestamps_are_utc_and_truncate_milliseconds() {
        assert_eq!(timestamp(T0), "2024-01-01 00:00:00");
        assert_eq!(timestamp(T1 - 1), "2026-07-31 23:59:59");
    }

    #[test]
    fn fixed_uses_javascript_tie_breaking_on_the_exact_float() {
        assert_eq!(fixed(2.675, 2), 2.67);
        assert_eq!(fixed(2_676.625, 2), 2_676.63);
    }
}
