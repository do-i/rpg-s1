//! Transactional economy, inn, and apothecary behavior shared by service UIs.

use std::{collections::BTreeMap, fmt};

use crate::{
    runtime_flags::RuntimeFlags,
    runtime_party::RuntimeParty,
    runtime_repository::RuntimeRepository,
    scenario_item::{ItemDefinition, MagicCoreItem},
    scenario_map::{ShopItem, ShopItemTag, ShopMetadata},
    scenario_recipe::{MagicCoreSize, RecipeDefinition},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecipeAvailability {
    Locked,
    MissingInputs,
    Unaffordable,
    UniqueOwned,
    OutputCap,
    Ready,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ServiceError {
    ZeroQuantity,
    Overflow,
    InsufficientGp,
    ItemCap,
    NotOwned,
    Locked,
    NotSellable,
    WrongCore,
    Recipe(RecipeAvailability),
    Repository(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroQuantity => formatter.write_str("quantity must be positive"),
            Self::Overflow => formatter.write_str("transaction total overflowed"),
            Self::InsufficientGp => formatter.write_str("not enough GP"),
            Self::ItemCap => formatter.write_str("item quantity cap reached"),
            Self::NotOwned => formatter.write_str("required item is not owned"),
            Self::Locked => formatter.write_str("item is locked"),
            Self::NotSellable => formatter.write_str("item cannot be sold"),
            Self::WrongCore => formatter.write_str("item is not a magic core"),
            Self::Recipe(state) => write!(formatter, "recipe is not ready: {state:?}"),
            Self::Repository(error) => formatter.write_str(error),
        }
    }
}

pub(crate) fn visible_stock<'a>(shop: &'a ShopMetadata, flags: &RuntimeFlags) -> Vec<&'a ShopItem> {
    shop.items
        .iter()
        .filter(|row| flags.is_set(row.unlock_flag()))
        .collect()
}

pub(crate) fn buy(
    repository: &mut RuntimeRepository,
    row: &ShopItem,
    quantity: u32,
) -> Result<u32, ServiceError> {
    require_quantity(quantity)?;
    let total = row
        .buy_price()
        .get()
        .checked_mul(quantity)
        .ok_or(ServiceError::Overflow)?;
    if repository.gp() < total {
        return Err(ServiceError::InsufficientGp);
    }
    if repository
        .item_count(row.id())
        .checked_add(quantity)
        .is_none_or(|value| value > repository.item_quantity_cap())
    {
        return Err(ServiceError::ItemCap);
    }

    let mut updated = repository.clone();
    updated.spend_gp(total).map_err(repository_error)?;
    let outcome = updated
        .add_item(row.id(), quantity)
        .map_err(repository_error)?;
    if outcome.added() != quantity {
        return Err(ServiceError::ItemCap);
    }
    if let ShopItem::Detailed(detail) = row {
        updated
            .add_tags(row.id(), detail.tags.iter().map(shop_tag))
            .map_err(repository_error)?;
    }
    *repository = updated;
    Ok(total)
}

pub(crate) fn sell_price(item: &ItemDefinition) -> Option<u32> {
    match item {
        ItemDefinition::Consumable(value) => (value.sell_price > 0).then_some(value.sell_price),
        ItemDefinition::Material(value) => (value.sell_price > 0).then_some(value.sell_price),
        ItemDefinition::Weapon(value) => value.sell_price.filter(|price| *price > 0),
        ItemDefinition::Shield(value) => value.sell_price.filter(|price| *price > 0),
        ItemDefinition::Helmet(value) => value.sell_price.filter(|price| *price > 0),
        ItemDefinition::Body(value) => value.sell_price.filter(|price| *price > 0),
        ItemDefinition::Accessory(value) => value.sell_price.filter(|price| *price > 0),
        ItemDefinition::Key(_) | ItemDefinition::MagicCore(_) => None,
    }
}

pub(crate) fn can_sell(repository: &RuntimeRepository, item: &ItemDefinition) -> bool {
    repository.item_count(item.id()) > 0
        && !repository.is_locked(item.id())
        && sell_price(item).is_some()
}

pub(crate) fn sell(
    repository: &mut RuntimeRepository,
    item: &ItemDefinition,
    quantity: u32,
) -> Result<u32, ServiceError> {
    require_quantity(quantity)?;
    if repository.is_locked(item.id()) {
        return Err(ServiceError::Locked);
    }
    let unit = sell_price(item).ok_or(ServiceError::NotSellable)?;
    if repository.item_count(item.id()) < quantity {
        return Err(ServiceError::NotOwned);
    }
    let total = unit.checked_mul(quantity).ok_or(ServiceError::Overflow)?;
    let mut updated = repository.clone();
    updated
        .remove_item(item.id(), quantity)
        .map_err(repository_error)?;
    if total > 0 {
        let _outcome = updated.add_gp(total).map_err(repository_error)?;
    }
    *repository = updated;
    Ok(total)
}

pub(crate) fn exchange_magic_core(
    repository: &mut RuntimeRepository,
    core: &ItemDefinition,
    quantity: u32,
) -> Result<u32, ServiceError> {
    require_quantity(quantity)?;
    let ItemDefinition::MagicCore(MagicCoreItem {
        exchange_rate, id, ..
    }) = core
    else {
        return Err(ServiceError::WrongCore);
    };
    if repository.item_count(id) < quantity {
        return Err(ServiceError::NotOwned);
    }
    let total = exchange_rate
        .get()
        .checked_mul(quantity)
        .ok_or(ServiceError::Overflow)?;
    let mut updated = repository.clone();
    updated
        .remove_item(id, quantity)
        .map_err(repository_error)?;
    let _outcome = updated.add_gp(total).map_err(repository_error)?;
    *repository = updated;
    Ok(total)
}

pub(crate) fn rest_at_inn(
    repository: &mut RuntimeRepository,
    party: &mut RuntimeParty,
    cost: u32,
) -> Result<(), ServiceError> {
    if repository.gp() < cost {
        return Err(ServiceError::InsufficientGp);
    }
    let mut updated_repository = repository.clone();
    let mut updated_party = party.clone();
    if cost > 0 {
        updated_repository
            .spend_gp(cost)
            .map_err(repository_error)?;
    }
    for member in updated_party.members_mut() {
        member.recover_at_inn();
    }
    *repository = updated_repository;
    *party = updated_party;
    Ok(())
}

pub(crate) fn recipe_availability(
    recipe: &RecipeDefinition,
    flags: &RuntimeFlags,
    repository: &RuntimeRepository,
) -> RecipeAvailability {
    if recipe
        .unlock_flag
        .as_ref()
        .is_some_and(|flag| !flags.is_set(flag))
    {
        return RecipeAvailability::Locked;
    }
    if recipe.unique_output && repository.item_count(&recipe.output.item) > 0 {
        return RecipeAvailability::UniqueOwned;
    }
    if !has_recipe_inputs(recipe, repository) {
        return RecipeAvailability::MissingInputs;
    }
    if repository.gp() < recipe.gp_cost {
        return RecipeAvailability::Unaffordable;
    }
    if repository
        .item_count(&recipe.output.item)
        .checked_add(recipe.output.qty.get())
        .is_none_or(|value| value > repository.item_quantity_cap())
    {
        return RecipeAvailability::OutputCap;
    }
    RecipeAvailability::Ready
}

pub(crate) fn craft(
    repository: &mut RuntimeRepository,
    flags: &RuntimeFlags,
    recipe: &RecipeDefinition,
) -> Result<(), ServiceError> {
    let availability = recipe_availability(recipe, flags, repository);
    if availability != RecipeAvailability::Ready {
        return Err(ServiceError::Recipe(availability));
    }
    let mut updated = repository.clone();
    if recipe.gp_cost > 0 {
        updated.spend_gp(recipe.gp_cost).map_err(repository_error)?;
    }
    for (id, quantity) in aggregated_inputs(recipe) {
        updated
            .remove_item(&id, quantity)
            .map_err(repository_error)?;
    }
    let outcome = updated
        .add_item(&recipe.output.item, recipe.output.qty.get())
        .map_err(repository_error)?;
    if outcome.added() != recipe.output.qty.get() {
        return Err(ServiceError::ItemCap);
    }
    *repository = updated;
    Ok(())
}

fn has_recipe_inputs(recipe: &RecipeDefinition, repository: &RuntimeRepository) -> bool {
    aggregated_inputs(recipe)
        .into_iter()
        .all(|(id, quantity)| repository.item_count(&id) >= quantity)
}

fn aggregated_inputs(recipe: &RecipeDefinition) -> BTreeMap<String, u32> {
    let mut inputs = BTreeMap::new();
    for item in &recipe.inputs.items {
        *inputs.entry(item.id.clone()).or_default() += item.qty.get();
    }
    for core in &recipe.inputs.mc {
        *inputs.entry(core_id(core.size).to_owned()).or_default() += core.qty.get();
    }
    inputs
}

const fn core_id(size: MagicCoreSize) -> &'static str {
    match size {
        MagicCoreSize::XS => "mc_xs",
        MagicCoreSize::S => "mc_s",
        MagicCoreSize::M => "mc_m",
        MagicCoreSize::L => "mc_l",
        MagicCoreSize::XL => "mc_xl",
    }
}

const fn shop_tag(tag: &ShopItemTag) -> &'static str {
    match tag {
        ShopItemTag::Consumable => "consumable",
        ShopItemTag::Recovery => "recovery",
        ShopItemTag::Status => "status",
    }
}

fn require_quantity(quantity: u32) -> Result<(), ServiceError> {
    (quantity > 0)
        .then_some(())
        .ok_or(ServiceError::ZeroQuantity)
}

fn repository_error(error: impl ToString) -> ServiceError {
    ServiceError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime_quest::{QuestStatus, quest_status},
        save_data::{
            NativeSaveEnvelope,
            tests::{fixture_balance, fixture_game},
        },
        scenario_item::{ItemCatalogFile, ItemStatus},
        scenario_map::MapMetadata,
        scenario_quest::{QuestCatalogFile, QuestDefinition},
        scenario_yaml,
    };

    fn ardel_shop() -> MapMetadata {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_shop_01.yaml"
        ))
        .unwrap()
    }

    fn item(id: &str) -> ItemDefinition {
        for document in [
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/items/consumables_recovery.yaml"
            ),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/magic_cores.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/key_items.yaml"),
        ] {
            let catalog: ItemCatalogFile = scenario_yaml::from_str(document).unwrap();
            if let Some(item) = catalog.entries().iter().find(|item| item.id() == id) {
                return item.clone();
            }
        }
        panic!("missing production item {id}");
    }

    fn quest(id: &str) -> QuestDefinition {
        let catalog: QuestCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/quests.yaml"
        ))
        .unwrap();
        catalog
            .entries()
            .iter()
            .find(|quest| quest.id == id)
            .unwrap()
            .clone()
    }

    #[test]
    fn stock_unlocks_and_buy_is_exact_tagged_and_atomic() {
        let mut game = fixture_game();
        let map = ardel_shop();
        let shop = map.shop.as_ref().unwrap();
        let rows = visible_stock(shop, game.flags());
        assert!(rows.iter().any(|row| row.id() == "potion"));
        assert!(!rows.iter().any(|row| row.id() == "hi_potion"));

        let potion = shop.items.iter().find(|row| row.id() == "potion").unwrap();
        let before = game.repository().clone();
        assert_eq!(
            buy(game.repository_mut(), potion, 99),
            Err(ServiceError::InsufficientGp)
        );
        assert_eq!(game.repository(), &before);

        let old_gp = game.repository().gp();
        let old_qty = game.repository().item_count("potion");
        assert_eq!(buy(game.repository_mut(), potion, 2), Ok(200));
        assert_eq!(game.repository().gp(), old_gp - 200);
        assert_eq!(game.repository().item_count("potion"), old_qty + 2);
        assert_eq!(
            game.repository().item_tags("potion").collect::<Vec<_>>(),
            ["consumable", "recovery"]
        );

        game.flags_mut().set("story_act2_started");
        assert!(
            visible_stock(shop, game.flags())
                .iter()
                .any(|row| row.id() == "hi_potion")
        );
    }

    #[test]
    fn selling_and_core_exchange_obey_ownership_lock_and_gp_rules() {
        let mut game = fixture_game();
        let potion = item("potion");
        let core = item("mc_s");
        let _outcome = game.repository_mut().add_item("mc_s", 2).unwrap();
        let before_core_gp = game.repository().gp();
        let rate = match &core {
            ItemDefinition::MagicCore(value) => value.exchange_rate.get(),
            _ => unreachable!(),
        };
        assert_eq!(
            exchange_magic_core(game.repository_mut(), &core, 2),
            Ok(rate * 2)
        );
        assert_eq!(game.repository().item_count("mc_s"), 0);
        assert_eq!(game.repository().gp(), before_core_gp + rate * 2);

        game.repository_mut().set_locked("potion", true);
        let locked = game.repository().clone();
        assert!(!can_sell(game.repository(), &potion));
        assert_eq!(
            sell(game.repository_mut(), &potion, 1),
            Err(ServiceError::Locked)
        );
        assert_eq!(game.repository(), &locked);
        assert_eq!(sell_price(&item("phoenix_wing")), None);
    }

    #[test]
    fn inn_recovers_knockout_mana_and_status_only_after_exact_payment() {
        let mut game = fixture_game();
        let protagonist_id = game.party().protagonist().unwrap().id().to_owned();
        let member = game.party_mut().member_mut(&protagonist_id).unwrap();
        member.apply_damage(u32::MAX);
        member.spend_mana(1);
        member.add_status(ItemStatus::Silence);
        let old_gp = game.repository().gp();
        let (repository, party) = game.repository_and_party_mut();
        rest_at_inn(repository, party, 50).unwrap();
        let member = game.party().protagonist().unwrap();
        assert_eq!(
            (member.health(), member.mana()),
            (member.max_health(), member.max_mana())
        );
        assert_eq!(member.status_effects().count(), 0);
        assert_eq!(game.repository().gp(), old_gp - 50);

        let remaining_gp = game.repository().gp();
        game.repository_mut().spend_gp(remaining_gp).unwrap();
        let before = game.clone();
        let (repository, party) = game.repository_and_party_mut();
        assert_eq!(
            rest_at_inn(repository, party, 50),
            Err(ServiceError::InsufficientGp)
        );
        assert_eq!(game, before);
    }

    #[test]
    fn recipe_classification_and_crafting_are_atomic_and_unique_safe() {
        let recipe: RecipeDefinition = scenario_yaml::from_str(
            "id: test\nscroll_name: Test\noutput: { item: remedy, qty: 1 }\nunique_output: true\ninputs:\n  items: [{ id: herb_red, qty: 2 }, { id: herb_red, qty: 1 }]\ngp_cost: 80\nunlock_flag: craft_open\n",
        ).unwrap();
        let mut game = fixture_game();
        assert_eq!(
            recipe_availability(&recipe, game.flags(), game.repository()),
            RecipeAvailability::Locked
        );
        game.flags_mut().set("craft_open");
        assert_eq!(
            recipe_availability(&recipe, game.flags(), game.repository()),
            RecipeAvailability::MissingInputs
        );
        let _outcome = game.repository_mut().add_item("herb_red", 3).unwrap();
        assert_eq!(
            recipe_availability(&recipe, game.flags(), game.repository()),
            RecipeAvailability::Ready
        );
        let old_gp = game.repository().gp();
        let flags = game.flags().clone();
        craft(game.repository_mut(), &flags, &recipe).unwrap();
        assert_eq!(game.repository().item_count("herb_red"), 0);
        assert_eq!(game.repository().item_count("remedy"), 1);
        assert_eq!(game.repository().gp(), old_gp - 80);
        let completed = game.repository().clone();
        assert_eq!(
            craft(game.repository_mut(), &flags, &recipe),
            Err(ServiceError::Recipe(RecipeAvailability::UniqueOwned))
        );
        assert_eq!(game.repository(), &completed);
    }

    #[test]
    fn service_results_and_dialogue_quest_flags_survive_native_save_roundtrip() {
        let mut game = fixture_game();
        let map = ardel_shop();
        let potion = map
            .shop
            .as_ref()
            .unwrap()
            .items
            .iter()
            .find(|row| row.id() == "potion")
            .unwrap();
        buy(game.repository_mut(), potion, 1).unwrap();
        let quest = quest("main_act1");
        game.flags_mut().set(&quest.completed_flag);
        let expected_gp = game.repository().gp();
        let expected_potions = game.repository().item_count("potion");

        let encoded =
            NativeSaveEnvelope::from_game_state(&game, "my_rpg_story", "1.0.0", 1, "Ardel")
                .unwrap()
                .encode()
                .unwrap();
        let (_, restored) =
            NativeSaveEnvelope::decode(&encoded, "my_rpg_story", "1.0.0", &fixture_balance())
                .unwrap();
        assert_eq!(restored.repository().gp(), expected_gp);
        assert_eq!(restored.repository().item_count("potion"), expected_potions);
        assert_eq!(
            quest_status(&quest, restored.flags()),
            QuestStatus::Completed
        );
    }
}
