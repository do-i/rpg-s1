//! The Forge: tempering gear into `Name +N` variants for GP and materials.
//!
//! Saves store item ids as plain strings, so an enhanced piece is its own catalog entry with an
//! id of `base+N` (or `base+N!` once a failed attempt has cracked it). The variants are generated
//! from the authored gear when the catalog loads; nothing about the save or item file formats
//! changes, and a save written by a build without the forge simply holds `+0` gear.

use std::collections::BTreeMap;

use crate::{
    field_menu_domain::FieldMenuCatalog,
    runtime_repository::RuntimeRepository,
    scenario_item::{AccessoryStats, HelmetStats, ItemDefinition},
    service_domain::{ServiceError, repository_error},
};

/// Separates a base item id from its tempering level.
const LEVEL_MARK: char = '+';
/// Trailing mark of a cracked piece.
const CRACK_MARK: char = '!';
/// Share of a mend's price, in percent of the item's value.
const MEND_PERCENT: u32 = 15;
/// Share of the GP spent on an upgrade that returns when the sell price is recomputed.
const SELL_BACK_PERCENT: u32 = 40;

/// A parsed enhanced-item id.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EnhancedId<'a> {
    pub base: &'a str,
    pub level: u32,
    pub cracked: bool,
}

pub(crate) fn parse_id(id: &str) -> Option<EnhancedId<'_>> {
    let (base, tail) = id.rsplit_once(LEVEL_MARK)?;
    let (digits, cracked) = tail
        .strip_suffix(CRACK_MARK)
        .map_or((tail, false), |digits| (digits, true));
    let level = digits.parse().ok()?;
    (!base.is_empty()).then_some(EnhancedId {
        base,
        level,
        cracked,
    })
}

/// The authored item an id derives from; plain ids are their own base.
pub(crate) fn base_id(id: &str) -> &str {
    parse_id(id).map_or(id, |parsed| parsed.base)
}

pub(crate) fn variant_id(base: &str, level: u32, cracked: bool) -> String {
    match (level, cracked) {
        (0, false) => base.to_owned(),
        (_, false) => format!("{base}{LEVEL_MARK}{level}"),
        (_, true) => format!("{base}{LEVEL_MARK}{level}{CRACK_MARK}"),
    }
}

/// The trade value tiers are priced from: the buy price, or twice the sell price for loot-only
/// gear. `None` makes the piece ineligible.
fn value(item: &ItemDefinition) -> Option<u32> {
    let (buy, sell) = match item {
        ItemDefinition::Weapon(v) => (v.buy_price, v.sell_price),
        ItemDefinition::Shield(v) => (v.buy_price, v.sell_price),
        ItemDefinition::Helmet(v) => (v.buy_price, v.sell_price),
        ItemDefinition::Body(v) => (v.buy_price, v.sell_price),
        ItemDefinition::Accessory(v) => (v.buy_price, v.sell_price),
        _ => return None,
    };
    buy.or(sell.map(|price| price * 2))
        .filter(|price| *price > 0)
}

/// Highest tempering level: cheap gear takes more, endgame gear less, unique gear least.
pub(crate) fn max_level(item: &ItemDefinition) -> u32 {
    let unique = match item {
        ItemDefinition::Weapon(v) => v.buy_price.is_none(),
        ItemDefinition::Shield(v) => v.buy_price.is_none(),
        ItemDefinition::Helmet(v) => v.buy_price.is_none(),
        ItemDefinition::Body(v) => v.buy_price.is_none(),
        ItemDefinition::Accessory(v) => v.buy_price.is_none(),
        _ => return 0,
    };
    let Some(price) = value(item) else { return 0 };
    if !has_primary_stat(item) {
        return 0;
    }
    match price {
        _ if unique => 2,
        0..=499 => 5,
        500..=1499 => 4,
        _ => 3,
    }
}

fn has_primary_stat(item: &ItemDefinition) -> bool {
    match item {
        ItemDefinition::Weapon(v) => {
            v.stats.strength.is_some() || v.stats.intelligence.is_some() || v.stats.dex.is_some()
        }
        ItemDefinition::Shield(_) | ItemDefinition::Helmet(_) | ItemDefinition::Body(_) => true,
        ItemDefinition::Accessory(v) => matches!(
            &v.stats,
            AccessoryStats::Attribute(s)
                if s.strength.is_some() || s.dex.is_some() || s.con.is_some()
                    || s.intelligence.is_some()
        ),
        _ => false,
    }
}

/// GP to reach `target_level` of a piece worth `price`.
pub(crate) fn gp_cost(price: u32, target_level: u32) -> u32 {
    (price / 4).max(25).saturating_mul(target_level)
}

/// The material consumed to reach `target_level`.
pub(crate) fn material_for(target_level: u32) -> &'static str {
    match target_level {
        0..=2 => "iron_plate",
        3..=4 => "golem_core",
        _ => "soul_fragment",
    }
}

fn mend_cost(price: u32) -> u32 {
    (price * MEND_PERCENT / 100).max(10)
}

fn bump(stat: &mut i32, amount: i32) {
    *stat += amount;
}

fn bump_primary(item: &mut ItemDefinition, amount: i32) {
    match item {
        ItemDefinition::Weapon(v) => {
            if let Some(stat) = v.stats.strength.as_mut() {
                bump(stat, amount);
            } else if let Some(stat) = v.stats.intelligence.as_mut() {
                bump(stat, amount);
            } else if let Some(stat) = v.stats.dex.as_mut() {
                bump(stat, amount);
            }
        }
        ItemDefinition::Shield(v) => bump(&mut v.stats.con, amount),
        ItemDefinition::Helmet(v) => match &mut v.stats {
            HelmetStats::Constitution(s) => bump(&mut s.con, amount),
            HelmetStats::Intelligence(s) => bump(&mut s.intelligence, amount),
        },
        ItemDefinition::Body(v) => bump(&mut v.stats.con, amount),
        ItemDefinition::Accessory(v) => {
            if let AccessoryStats::Attribute(s) = &mut v.stats {
                if let Some(stat) = s.strength.as_mut() {
                    bump(stat, amount);
                } else if let Some(stat) = s.intelligence.as_mut() {
                    bump(stat, amount);
                } else if let Some(stat) = s.con.as_mut() {
                    bump(stat, amount);
                } else if let Some(stat) = s.dex.as_mut() {
                    bump(stat, amount);
                }
            }
        }
        _ => {}
    }
}

/// The stat a tempering level raises, as a short label and its current value.
pub(crate) fn primary_stat(item: &ItemDefinition) -> Option<(&'static str, i32)> {
    match item {
        ItemDefinition::Weapon(v) => v
            .stats
            .strength
            .map(|n| ("STR", n))
            .or(v.stats.intelligence.map(|n| ("INT", n)))
            .or(v.stats.dex.map(|n| ("DEX", n))),
        ItemDefinition::Shield(v) => Some(("CON", v.stats.con)),
        ItemDefinition::Helmet(v) => Some(match &v.stats {
            HelmetStats::Constitution(s) => ("CON", s.con),
            HelmetStats::Intelligence(s) => ("INT", s.intelligence),
        }),
        ItemDefinition::Body(v) => Some(("CON", v.stats.con)),
        ItemDefinition::Accessory(v) => match &v.stats {
            AccessoryStats::Attribute(s) => s
                .strength
                .map(|n| ("STR", n))
                .or(s.intelligence.map(|n| ("INT", n)))
                .or(s.con.map(|n| ("CON", n)))
                .or(s.dex.map(|n| ("DEX", n))),
            _ => None,
        },
        _ => None,
    }
}

/// The five kinds of equipment the forge's left column groups by.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ForgeType {
    Weapon,
    Shield,
    Helmet,
    Body,
    Accessory,
}

impl ForgeType {
    pub(crate) const ALL: [Self; 5] = [
        Self::Weapon,
        Self::Shield,
        Self::Helmet,
        Self::Body,
        Self::Accessory,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Shield => "Shield",
            Self::Helmet => "Helmet",
            Self::Body => "Body",
            Self::Accessory => "Accessory",
        }
    }

    pub(crate) fn of(item: &ItemDefinition) -> Option<Self> {
        match item {
            ItemDefinition::Weapon(_) => Some(Self::Weapon),
            ItemDefinition::Shield(_) => Some(Self::Shield),
            ItemDefinition::Helmet(_) => Some(Self::Helmet),
            ItemDefinition::Body(_) => Some(Self::Body),
            ItemDefinition::Accessory(_) => Some(Self::Accessory),
            _ => None,
        }
    }
}

/// Every owned piece of one equipment type the smith can still work on, in id order.
pub(crate) fn forgeable_of(
    catalog: &FieldMenuCatalog,
    repository: &RuntimeRepository,
    kind: ForgeType,
) -> Vec<String> {
    let mut ids: Vec<String> = repository
        .item_counts()
        .filter(|(id, count)| {
            *count > 0
                && quote(catalog, id).is_some()
                && catalog.item(id).and_then(ForgeType::of) == Some(kind)
        })
        .map(|(id, _)| id.to_owned())
        .collect();
    ids.sort_unstable();
    ids
}

/// GP paid across every step up to `level`, which feeds the sell-back bonus.
fn spent_gp(price: u32, level: u32) -> u32 {
    (1..=level).fold(0, |total, step| total.saturating_add(gp_cost(price, step)))
}

/// Builds the `base +level` entry: primary stat up by `level` (one less when cracked), the sell
/// price raised by a share of the GP spent, and no shop price.
fn variant(base: &ItemDefinition, level: u32, cracked: bool) -> Option<ItemDefinition> {
    let price = value(base)?;
    let mut item = base.clone();
    let amount = i32::try_from(level).ok()? - i32::from(cracked);
    bump_primary(&mut item, amount);
    let id = variant_id(base.id(), level, cracked);
    let suffix = if cracked {
        format!(" +{level} (Cracked)")
    } else {
        format!(" +{level}")
    };
    let sell_bonus = spent_gp(price, level) * SELL_BACK_PERCENT / 100;
    macro_rules! apply {
        ($v:expr) => {{
            $v.id = id;
            $v.name = format!("{}{suffix}", $v.name);
            $v.buy_price = None;
            $v.sell_price = $v.sell_price.map(|sell| sell + sell_bonus);
            $v.description = if cracked {
                format!(
                    "{} A hairline crack runs through it; the smith can mend it.",
                    $v.description
                )
            } else {
                format!("{} Tempered at the forge.", $v.description)
            };
        }};
    }
    match &mut item {
        ItemDefinition::Weapon(v) => apply!(v),
        ItemDefinition::Shield(v) => apply!(v),
        ItemDefinition::Helmet(v) => apply!(v),
        ItemDefinition::Body(v) => apply!(v),
        ItemDefinition::Accessory(v) => apply!(v),
        _ => return None,
    }
    Some(item)
}

/// Adds every tempered and cracked variant to a freshly loaded item table, listing them right
/// after their base so inventories sort them together.
pub(crate) fn expand_catalog(
    items: &mut BTreeMap<String, ItemDefinition>,
    order: &mut Vec<String>,
) {
    let mut expanded = Vec::with_capacity(order.len());
    let mut added = Vec::new();
    for id in order.drain(..) {
        expanded.push(id.clone());
        let Some(base) = items.get(&id) else { continue };
        let top = max_level(base);
        if top == 0 || parse_id(&id).is_some() {
            continue;
        }
        let tempered = (1..=top).filter_map(|level| variant(base, level, false));
        let cracked = (0..top).filter_map(|level| variant(base, level, true));
        for entry in tempered.chain(cracked) {
            expanded.push(entry.id().to_owned());
            added.push(entry);
        }
    }
    for entry in added {
        items.insert(entry.id().to_owned(), entry);
    }
    *order = expanded;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ForgeKind {
    Enhance,
    Mend,
}

/// What the smith asks for one piece, or `None` when nothing more can be done to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ForgeQuote {
    pub kind: ForgeKind,
    pub target_id: String,
    pub gp: u32,
    pub material: Option<(&'static str, u32)>,
    /// Percent chances of (great, success, fizzle, backfire); empty odds for a mend.
    pub odds: Option<Odds>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Odds {
    pub great: u32,
    pub success: u32,
    pub fizzle: u32,
    pub backfire: u32,
}

/// Odds for reaching `target_level`: every level past +2 trades 5% of success for fizzle.
pub(crate) fn odds(target_level: u32) -> Odds {
    let shift = 5 * target_level.saturating_sub(2);
    Odds {
        great: 20,
        success: 60 - shift,
        fizzle: 15 + shift,
        backfire: 5,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ForgeOutcome {
    /// One level up.
    Success(String),
    /// Two levels up for the price of one (capped at the piece's maximum).
    Great(String),
    /// Materials lost, half the GP returned.
    Fizzle,
    /// The piece cracks: a stat point lower until mended.
    Backfire(String),
    Mended(String),
}

pub(crate) fn quote(catalog: &FieldMenuCatalog, item_id: &str) -> Option<ForgeQuote> {
    let parsed = parse_id(item_id).unwrap_or(EnhancedId {
        base: item_id,
        level: 0,
        cracked: false,
    });
    let base = catalog.item(parsed.base)?;
    let top = max_level(base);
    let price = value(base)?;
    if top == 0 || parsed.level > top {
        return None;
    }
    if parsed.cracked {
        return Some(ForgeQuote {
            kind: ForgeKind::Mend,
            target_id: variant_id(parsed.base, parsed.level, false),
            gp: mend_cost(price),
            material: None,
            odds: None,
        });
    }
    if parsed.level >= top {
        return None;
    }
    let target = parsed.level + 1;
    Some(ForgeQuote {
        kind: ForgeKind::Enhance,
        target_id: variant_id(parsed.base, target, false),
        gp: gp_cost(price, target),
        material: Some((material_for(target), 1)),
        odds: Some(odds(target)),
    })
}

/// Picks the outcome of one attempt from a single RNG draw.
pub(crate) fn roll(draw: u64, target_level: u32) -> Roll {
    let odds = odds(target_level);
    let pick = u32::try_from(draw % 100).unwrap_or(0);
    if pick < odds.great {
        Roll::Great
    } else if pick < odds.great + odds.success {
        Roll::Success
    } else if pick < odds.great + odds.success + odds.fizzle {
        Roll::Fizzle
    } else {
        Roll::Backfire
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Roll {
    Great,
    Success,
    Fizzle,
    Backfire,
}

/// Runs one forge transaction on an unlocked, owned piece. `draw` is consumed only by an
/// enhancement; every failed check leaves the repository untouched.
/// Every precondition of a strike, without touching the repository. Callers use it to decide
/// whether a strike will roll before they take a draw from the saved gameplay RNG.
pub(crate) fn check(
    repository: &RuntimeRepository,
    catalog: &FieldMenuCatalog,
    item_id: &str,
) -> Result<ForgeQuote, ServiceError> {
    let quote = quote(catalog, item_id).ok_or(ServiceError::NotEnhanceable)?;
    if repository.item_count(item_id) == 0 {
        return Err(ServiceError::NotOwned);
    }
    if repository.is_locked(item_id) {
        return Err(ServiceError::Locked);
    }
    if repository.gp() < quote.gp {
        return Err(ServiceError::InsufficientGp);
    }
    if let Some((material, count)) = quote.material
        && repository.item_count(material) < count
    {
        return Err(ServiceError::MissingMaterial);
    }
    Ok(quote)
}

pub(crate) fn forge(
    repository: &mut RuntimeRepository,
    catalog: &FieldMenuCatalog,
    item_id: &str,
    draw: u64,
) -> Result<ForgeOutcome, ServiceError> {
    let quote = check(repository, catalog, item_id)?;

    let parsed = parse_id(item_id).unwrap_or(EnhancedId {
        base: item_id,
        level: 0,
        cracked: false,
    });
    let top = max_level(
        catalog
            .item(parsed.base)
            .ok_or(ServiceError::NotEnhanceable)?,
    );
    let (outcome, result_id) = match quote.kind {
        ForgeKind::Mend => (
            ForgeOutcome::Mended(quote.target_id.clone()),
            Some(quote.target_id.clone()),
        ),
        ForgeKind::Enhance => match roll(draw, parsed.level + 1) {
            Roll::Success => (
                ForgeOutcome::Success(quote.target_id.clone()),
                Some(quote.target_id.clone()),
            ),
            Roll::Great => {
                let id = variant_id(parsed.base, (parsed.level + 2).min(top), false);
                (ForgeOutcome::Great(id.clone()), Some(id))
            }
            Roll::Fizzle => (ForgeOutcome::Fizzle, None),
            Roll::Backfire => {
                let id = variant_id(parsed.base, parsed.level, true);
                (ForgeOutcome::Backfire(id.clone()), Some(id))
            }
        },
    };
    if let Some(id) = &result_id
        && repository.item_count(id) >= repository.item_quantity_cap()
    {
        return Err(ServiceError::ItemCap);
    }

    let mut updated = repository.clone();
    let spend = if outcome == ForgeOutcome::Fizzle {
        quote.gp - quote.gp / 2
    } else {
        quote.gp
    };
    updated.spend_gp(spend).map_err(repository_error)?;
    if let Some((material, count)) = quote.material {
        updated
            .remove_item(material, count)
            .map_err(repository_error)?;
    }
    if let Some(id) = result_id {
        updated.remove_item(item_id, 1).map_err(repository_error)?;
        let added = updated.add_item(&id, 1).map_err(repository_error)?;
        if added.added() != 1 {
            return Err(ServiceError::ItemCap);
        }
    }
    *repository = updated;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu_domain::tests::{catalog, game};

    #[test]
    fn ids_round_trip_and_plain_ids_are_their_own_base() {
        assert_eq!(variant_id("iron_sword", 0, false), "iron_sword");
        assert_eq!(variant_id("iron_sword", 2, false), "iron_sword+2");
        assert_eq!(variant_id("iron_sword", 2, true), "iron_sword+2!");
        assert_eq!(
            parse_id("iron_sword+2!"),
            Some(EnhancedId {
                base: "iron_sword",
                level: 2,
                cracked: true
            })
        );
        assert_eq!(parse_id("iron_sword"), None);
        assert_eq!(base_id("iron_sword+3"), "iron_sword");
        assert_eq!(base_id("potion"), "potion");
    }

    #[test]
    fn caps_follow_price_tiers() {
        let catalog = catalog();
        for (id, cap) in [
            ("short_sword", 5),
            ("iron_sword", 5),
            ("steel_sword", 4),
            ("flamebrand", 3),
            ("potion", 0),
        ] {
            assert_eq!(max_level(catalog.item(id).unwrap()), cap, "{id}");
        }
        assert!(catalog.item("iron_sword+5").is_some());
        assert!(catalog.item("iron_sword+6").is_none());
        assert!(catalog.item("flamebrand+3").is_some());
        assert!(catalog.item("flamebrand+4").is_none());
    }

    #[test]
    fn variants_raise_the_primary_stat_and_cracks_cost_one_point() {
        let catalog = catalog();
        let strength = |id: &str| match catalog.item(id).unwrap() {
            ItemDefinition::Weapon(v) => v.stats.strength.unwrap(),
            other => panic!("{other:?}"),
        };
        assert_eq!(strength("iron_sword"), 4);
        assert_eq!(strength("iron_sword+3"), 7);
        assert_eq!(strength("iron_sword+3!"), 6);
        assert_eq!(strength("iron_sword+0!"), 3);
        let ItemDefinition::Weapon(plus) = catalog.item("iron_sword+2").unwrap() else {
            panic!()
        };
        assert_eq!(plus.name, "Iron Sword +2");
        assert_eq!(plus.buy_price, None);
        assert!(plus.sell_price.unwrap() > 175);
    }

    #[test]
    fn every_forge_material_exists_and_odds_sum_to_one_hundred() {
        let catalog = catalog();
        for level in 1..=5 {
            assert!(catalog.item(material_for(level)).is_some());
            let odds = odds(level);
            assert_eq!(odds.great + odds.success + odds.fizzle + odds.backfire, 100);
        }
        assert_eq!(odds(5).success, 45);
    }

    #[test]
    fn rolls_partition_the_draw_space() {
        assert_eq!(roll(0, 1), Roll::Great);
        assert_eq!(roll(19, 1), Roll::Great);
        assert_eq!(roll(20, 1), Roll::Success);
        assert_eq!(roll(79, 1), Roll::Success);
        assert_eq!(roll(80, 1), Roll::Fizzle);
        assert_eq!(roll(95, 1), Roll::Backfire);
        // At +5 the fizzle band widens at success's expense.
        assert_eq!(roll(65, 5), Roll::Fizzle);
        assert_eq!(roll(65, 1), Roll::Success);
    }

    fn funded(id: &str, gp: u32) -> crate::game_state::GameState {
        let mut game = game([]);
        let repo = game.repository_mut();
        let _ = repo.add_gp(gp).unwrap();
        let _ = repo.add_item(id, 1).unwrap();
        let _ = repo.add_item("iron_plate", 3).unwrap();
        game
    }

    #[test]
    fn quote_prices_the_next_level_and_stops_at_the_cap() {
        let catalog = catalog();
        let q = quote(&catalog, "iron_sword").unwrap();
        assert_eq!(
            (q.kind, q.target_id.as_str(), q.gp),
            (ForgeKind::Enhance, "iron_sword+1", 87)
        );
        assert_eq!(q.material, Some(("iron_plate", 1)));
        assert!(quote(&catalog, "iron_sword+5").is_none());
        assert!(quote(&catalog, "potion").is_none());
        let mend = quote(&catalog, "iron_sword+2!").unwrap();
        assert_eq!(
            (mend.kind, mend.target_id.as_str(), mend.material),
            (ForgeKind::Mend, "iron_sword+2", None)
        );
    }

    #[test]
    fn each_outcome_moves_exactly_the_right_items_and_gp() {
        let catalog = catalog();
        for (draw, expect, item, gp_after, plate_after) in [
            (
                50,
                ForgeOutcome::Success("iron_sword+1".into()),
                "iron_sword+1",
                913,
                2,
            ),
            (
                5,
                ForgeOutcome::Great("iron_sword+2".into()),
                "iron_sword+2",
                913,
                2,
            ),
            (85, ForgeOutcome::Fizzle, "iron_sword", 956, 2),
            (
                99,
                ForgeOutcome::Backfire("iron_sword+0!".into()),
                "iron_sword+0!",
                913,
                2,
            ),
        ] {
            let mut game = funded("iron_sword", 1000);
            let repo = game.repository_mut();
            assert_eq!(forge(repo, &catalog, "iron_sword", draw), Ok(expect));
            assert_eq!(repo.item_count(item), 1, "{item}");
            assert_eq!(repo.gp(), gp_after);
            assert_eq!(repo.item_count("iron_plate"), plate_after);
            let held: u32 = repo
                .item_counts()
                .filter(|(id, _)| base_id(id) == "iron_sword")
                .map(|(_, n)| n)
                .sum();
            assert_eq!(held, 1, "the piece is swapped, never duplicated");
        }
    }

    #[test]
    fn a_cracked_piece_mends_for_gp_only() {
        let catalog = catalog();
        let mut game = funded("iron_sword+2!", 100);
        let repo = game.repository_mut();
        assert_eq!(
            forge(repo, &catalog, "iron_sword+2!", 0),
            Ok(ForgeOutcome::Mended("iron_sword+2".into()))
        );
        assert_eq!(repo.item_count("iron_sword+2"), 1);
        assert_eq!(repo.item_count("iron_plate"), 3);
        assert_eq!(repo.gp(), 100 - 52);
    }

    #[test]
    fn failed_checks_change_nothing() {
        let catalog = catalog();
        for (setup, id, expected) in [
            (
                funded("iron_sword", 10),
                "iron_sword",
                ServiceError::InsufficientGp,
            ),
            (
                funded("iron_sword+5", 9999),
                "iron_sword+5",
                ServiceError::NotEnhanceable,
            ),
            (
                funded("potion", 9999),
                "potion",
                ServiceError::NotEnhanceable,
            ),
            (
                funded("iron_sword", 9999),
                "steel_sword",
                ServiceError::NotOwned,
            ),
        ] {
            let mut game = setup;
            let repo = game.repository_mut();
            let before = repo.clone();
            assert_eq!(forge(repo, &catalog, id, 50), Err(expected));
            assert_eq!(*repo, before);
        }
        let mut game = funded("iron_sword", 9999);
        let repo = game.repository_mut();
        repo.remove_item("iron_plate", 3).unwrap();
        let before = repo.clone();
        assert_eq!(
            forge(repo, &catalog, "iron_sword", 50),
            Err(ServiceError::MissingMaterial)
        );
        assert_eq!(*repo, before);
        let _ = repo.add_item("iron_plate", 1).unwrap();
        repo.set_locked("iron_sword", true);
        assert_eq!(
            forge(repo, &catalog, "iron_sword", 50),
            Err(ServiceError::Locked)
        );
    }
}
